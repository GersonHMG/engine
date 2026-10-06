// Sysmic Robotics — RoboCup SSL Engine (Rust)

#[path = "utils/types.rs"]
mod types;
#[path = "protobuf/proto.rs"]
mod proto;
mod receiver;
#[path = "receiver/game_controller.rs"]
mod game_controller;
mod world;
mod motion;
mod sender;
mod lua_interface;
#[path = "logger/logger.rs"]
mod logger;
mod gui;
mod config;

use std::sync::{Arc, RwLock, Mutex};
use std::time::{Duration, Instant};

use tracing::{info, warn};

use crate::game_controller::GameState;
use crate::lua_interface::LuaInterface;
use crate::lua_interface::ScriptExecState;
use crate::receiver::vision;
use crate::sender::radio::Radio;
use crate::sender::simulator::console as simulator_console;
use crate::sender::simulator::SimLink;
use crate::world::World;
use crate::logger::Logger;
use crate::types::{KickerCommand, MotionCommand};
use crate::gui::{EngineApp, EngineCommand, GuiChannels, LuaDrawCmd, LuaScriptStatusUpdate, SimulatorSettings, VisionUpdate};
use crate::gui::toolbar::ScriptStatus;
use crate::config::load_field_config_from_exe;


#[derive(Default)]
struct VisionState {
    handle: Option<tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>>>,
    tx: Option<tokio::sync::mpsc::Sender<vision::VisionCommand>>,
    ip: String,
    port: u16,
}

impl VisionState {
    /// Starts the vision receiver on `ip:port`, replacing a running one.
    fn start(&mut self, world: &Arc<RwLock<World>>, gui_tx: &tokio::sync::mpsc::Sender<VisionUpdate>) {
        self.stop();
        let (tx, rx) = tokio::sync::mpsc::channel(32);
        self.tx = Some(tx);

        let (ip, port) = (self.ip.clone(), self.port);
        let world = Arc::clone(world);
        let gui_tx = gui_tx.clone();
        self.handle = Some(tokio::spawn(async move {
            if let Err(e) = vision::run_vision(ip, port, world, gui_tx, rx).await {
                warn!("Vision task error: {e}");
            }
            Ok(())
        }));
    }

    fn stop(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
        self.tx = None;
    }
}

/// Switches between the in-process simulator and the vision receiver
/// (grSim or real robots). The world is cleared so nothing from the
/// previous source lingers.
fn switch_simulator(
    enabled: bool,
    field_config: crate::config::FieldConfig,
    radio: &Mutex<Radio>,
    world: &Arc<RwLock<World>>,
    vision_state: &Mutex<VisionState>,
    vision_gui_tx: &tokio::sync::mpsc::Sender<VisionUpdate>,
) {
    let mut radio = radio.lock().unwrap_or_else(|e| e.into_inner());
    if radio.has_simulator() == enabled {
        return;
    }

    let mut vision = vision_state.lock().unwrap_or_else(|e| e.into_inner());
    if enabled {
        vision.stop();
        radio.attach_simulator(SimLink::new(crate::config::load_simulator_config(field_config)));
    } else {
        radio.detach_simulator();
        vision.start(world, vision_gui_tx);
    }
    world.write().unwrap_or_else(|e| e.into_inner()).clear();
    info!("Simulator {}", if enabled { "on" } else { "off, vision receiver restarted" });
}

/// Real time to spend per tick, or `None` to run flat out. Only the
/// simulator can run faster than real time.
fn frame_duration(simulator: SimulatorSettings) -> Option<Duration> {
    if !simulator.enabled {
        Some(TICK)
    } else if simulator.speed > 0.0 {
        Some(TICK.div_f64(simulator.speed))
    } else {
        None
    }
}

/// One engine tick (~60 FPS). With the simulator this is also the
/// simulated time each tick advances.
const TICK: Duration = Duration::from_micros(16_667);

/// Vision frame rate reported to the GUI in simulator mode (one frame per tick).
const SIM_FRAME_RATE: u32 = 60;

/// Command line: `engine [--sim] [--speed <x>] [script.lua]`. The
/// simulator can also be switched on and off from the toolbar.
struct LaunchArgs {
    script: Option<String>,
    /// Start with the in-process simulator instead of grSim or the radio.
    simulator: bool,
    /// Simulated seconds per real second; 0 = as fast as possible.
    speed: f64,
}

impl LaunchArgs {
    fn parse() -> Self {
        let mut launch = Self { script: None, simulator: false, speed: 1.0 };
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--sim" => launch.simulator = true,
                "--speed" => match args.next().and_then(|v| v.parse::<f64>().ok()) {
                    Some(speed) if speed >= 0.0 => launch.speed = speed,
                    _ => warn!("--speed needs a number >= 0 (0 = as fast as possible), using 1"),
                },
                _ if launch.script.is_none() => launch.script = Some(arg),
                _ => warn!("Ignoring extra argument: {arg}"),
            }
        }
        launch
    }

    fn simulator_settings(&self) -> SimulatorSettings {
        SimulatorSettings { enabled: self.simulator, speed: self.speed }
    }
}

fn map_script_state(state: ScriptExecState) -> ScriptStatus {
    match state {
        ScriptExecState::NoScript => ScriptStatus::NoScript,
        ScriptExecState::Running => ScriptStatus::Running,
        ScriptExecState::Paused => ScriptStatus::Paused,
        ScriptExecState::Failed => ScriptStatus::Failed,
    }
}

fn main() -> iced::Result {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let launch = LaunchArgs::parse();
    let simulator = launch.simulator_settings();
    let field_config = load_field_config_from_exe();

    // Create channels between GUI and engine
    let (vision_tx, vision_rx) = tokio::sync::mpsc::channel::<VisionUpdate>(256);
    let (lua_draw_tx, lua_draw_rx) = tokio::sync::mpsc::channel::<Vec<LuaDrawCmd>>(256);
    let (lua_status_tx, lua_status_rx) = tokio::sync::mpsc::channel::<LuaScriptStatusUpdate>(64);
    let (lua_log_tx, lua_log_rx) = tokio::sync::mpsc::channel::<String>(512);
    let (command_tx, command_rx) = tokio::sync::mpsc::channel::<EngineCommand>(256);

    let gui_channels = GuiChannels {
        vision_rx,
        lua_draw_rx,
        lua_status_rx,
        lua_log_rx,
        command_tx: command_tx.clone(),
    };

    // Spawn the engine in a background tokio runtime
    let command_rx = Arc::new(Mutex::new(Some(command_rx)));
    let vision_tx_clone = vision_tx.clone();
    let lua_draw_tx_clone = lua_draw_tx.clone();
    let lua_status_tx_clone = lua_status_tx.clone();
    let lua_log_tx_clone = lua_log_tx.clone();
    let command_rx_clone = command_rx.clone();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
        rt.block_on(async move {
            let rx = command_rx_clone.lock().unwrap().take().expect("command_rx already taken");
            run_engine(
                vision_tx_clone,
                lua_draw_tx_clone,
                lua_status_tx_clone,
                lua_log_tx_clone,
                rx,
                field_config,
                launch,
            )
            .await;
        });
    });

    // Wrap gui_channels so the boot closure can be Fn (not FnOnce)
    let gui_channels = std::sync::Arc::new(std::sync::Mutex::new(Some(gui_channels)));

    // Run Iced daemon on the main thread (multi-window)
    iced::daemon(
        move || {
            let channels = gui_channels.lock().unwrap().take()
                .expect("GUI channels already consumed");
            EngineApp::boot(channels, field_config, simulator)
        },
        EngineApp::update,
        EngineApp::view,
    )
    .title(EngineApp::title)
    .theme(EngineApp::theme)
    .subscription(EngineApp::subscription)
    .run()
}

async fn run_engine(
    vision_gui_tx: tokio::sync::mpsc::Sender<VisionUpdate>,
    lua_draw_gui_tx: tokio::sync::mpsc::Sender<Vec<LuaDrawCmd>>,
    lua_status_gui_tx: tokio::sync::mpsc::Sender<LuaScriptStatusUpdate>,
    lua_log_gui_tx: tokio::sync::mpsc::Sender<String>,
    mut command_rx: tokio::sync::mpsc::Receiver<EngineCommand>,
    field_config: crate::config::FieldConfig,
    launch: LaunchArgs,
) {
    // Configuration Defaults
    let vision_ip = "224.5.23.2".to_string();
    let vision_port = 10020u16;
    
    let blue_team_size = 6;
    let yellow_team_size = 6;

    let use_radio = false;
    let radio_port = "/dev/ttyUSB0".to_string();
    let radio_baud = 115200;

    // Shared state
    let world = Arc::new(RwLock::new(World::new(
        blue_team_size,
        yellow_team_size,
        field_config.length_m,
        field_config.width_m,
    )));
    let game_state = Arc::new(Mutex::new(GameState::new()));
    let radio = Arc::new(Mutex::new(Radio::new(use_radio, &radio_port, radio_baud)));
    let vision_state = Arc::new(Mutex::new(VisionState {
        ip: vision_ip,
        port: vision_port,
        ..Default::default()
    }));
    let mut simulator = launch.simulator_settings();
    let logger = Arc::new(Mutex::new(Logger::new()));

    // Console replies share the Lua log channel, so they show in the same panel.
    let console_tx = lua_log_gui_tx.clone();
    let lua_iface = Arc::new(Mutex::new(LuaInterface::new(
        Arc::clone(&radio),
        Arc::clone(&world),
        Arc::clone(&game_state),
        Some(lua_log_gui_tx),
    )));

    let last_script_path = Arc::new(Mutex::new(String::new()));
    let mut last_script_state = ScriptExecState::NoScript;
    let _ = lua_status_gui_tx.try_send(LuaScriptStatusUpdate {
        status: map_script_state(last_script_state),
        script_path: None,
    });

    // World source: the in-process simulator, or the vision receiver
    if simulator.enabled {
        switch_simulator(true, field_config, &radio, &world, &vision_state, &vision_gui_tx);
    } else {
        vision_state.lock().unwrap().start(&world, &vision_gui_tx);
    }

    // Spawn Game Controller receiver task
    let game_state_for_ref = Arc::clone(&game_state);
    let _ref_handle = tokio::spawn(async move {
        if let Err(e) = game_controller::run_game_controller("224.5.23.1", 10003, game_state_for_ref).await {
            warn!("GameController task error: {e}");
        }
    });

    // Run script from command line arg if provided
    if let Some(script_path) = launch.script.clone() {
        let state = {
            let mut lua = lua_iface.lock().unwrap();
            lua.run_script(&script_path)
        };
        last_script_state = state;
        let _ = lua_status_gui_tx.try_send(LuaScriptStatusUpdate {
            status: map_script_state(state),
            script_path: Some(script_path.clone()),
        });
        let mut last = last_script_path.lock().unwrap();
        *last = script_path;
    }

    // Main update loop: one tick per TICK of real time; with the simulator
    // the speed setting scales it.
    info!("Engine started. Simulator: {}.", if simulator.enabled { "on" } else { "off" });

    loop {
        let frame_start = Instant::now();

        // Remove robots that have not updated in 60*4 frames (~4s).
        {
            let mut w = match world.write() {
                Ok(guard) => guard,
                Err(poisoned) => {
                    warn!("World lock poisoned, recovering");
                    poisoned.into_inner()
                }
            };
            w.prune_stale_robots();
        }

        // Receiver snapshot at frame start for logging.
        let (log_blue_robots, log_yellow_robots, log_ball) = {
            let w = match world.read() {
                Ok(guard) => guard,
                Err(poisoned) => {
                    warn!("World lock poisoned, recovering");
                    poisoned.into_inner()
                }
            };
            (w.get_blue_team_state(), w.get_yellow_team_state(), w.get_ball_state())
        };

        // Process GUI commands
        while let Ok(cmd) = command_rx.try_recv() {
            match cmd {
                EngineCommand::UpdateVisionConnection { ip, port } => {
                    let mut vs = vision_state.lock().unwrap();
                    if vs.ip == ip && vs.port == port && (vs.handle.is_some() || simulator.enabled) {
                        continue;
                    }
                    vs.ip = ip.clone();
                    vs.port = port;
                    if simulator.enabled {
                        info!("Vision set to {}:{}, used when the simulator is turned off", ip, port);
                    } else {
                        vs.start(&world, &vision_gui_tx);
                        info!("Restarted vision task with {}:{}", ip, port);
                    }
                }
                EngineCommand::TeleportRobot { id, team, x, y, orientation } => {
                    radio.lock().unwrap().teleport_robot(id, team, x, y, orientation);
                }
                EngineCommand::TeleportBall { x, y } => {
                    radio.lock().unwrap().teleport_ball(x, y);
                }
                EngineCommand::ConsoleCommand(line) => {
                    let reply = {
                        let mut r = radio.lock().unwrap();
                        let mut w = world.write().unwrap_or_else(|e| e.into_inner());
                        simulator_console::execute(&line, r.simulator_mut(), &mut w)
                    };
                    for out in std::iter::once(format!("> {line}")).chain(reply) {
                        let _ = console_tx.try_send(out);
                    }
                }
                EngineCommand::SetSimulator(settings) => {
                    switch_simulator(settings.enabled, field_config, &radio, &world, &vision_state, &vision_gui_tx);
                    simulator = settings;
                }
                EngineCommand::UpdateRadioConfig { use_radio, port_name, baud_rate } => {
                    let mut r = radio.lock().unwrap();
                    r.reconfigure(use_radio, &port_name, baud_rate);
                    info!("Radio reconfigured: use_radio={}, port={}, baud={}", use_radio, port_name, baud_rate);
                }
                EngineCommand::UpdateTrackerConfig { enabled, process_noise_p, process_noise_v, measurement_noise } => {
                    let tx = {
                        let vs = vision_state.lock().unwrap();
                        vs.tx.clone()
                    };
                    if let Some(tx) = tx {
                        let cmd = vision::VisionCommand::UpdateTrackerConfig {
                            enabled,
                            process_noise_p,
                            process_noise_v,
                            measurement_noise,
                        };
                        let _ = tx.try_send(cmd);
                    }
                }
                EngineCommand::StartRecording { filename } => {
                    let mut l = logger.lock().unwrap();
                    if !l.is_logging() {
                        l.start_logging(Some(&filename));
                        info!("Started recording to {}", filename);
                    }
                }
                EngineCommand::StopRecording => {
                    let mut l = logger.lock().unwrap();
                    if l.is_logging() {
                        l.stop_logging();
                        info!("Stopped recording");
                    }
                }
                EngineCommand::SendRobotCommand { id, team, vx, vy, omega } => {
                    let mut r = radio.lock().unwrap();
                    let cmd = MotionCommand { id, team, vx: Some(vx), vy: Some(vy), angular: Some(omega) };
                    r.add_motion_command(cmd);
                }
                EngineCommand::SendKickCommand { id, team } => {
                    let mut r = radio.lock().unwrap();
                    let mut kicker = KickerCommand::new(id, team);
                    kicker.kick_x = true;
                    r.add_kicker_command(kicker);
                }
                EngineCommand::LoadScript { path } => {
                    let state = {
                        let mut lua = lua_iface.lock().unwrap();
                        lua.run_script(&path)
                    };
                    last_script_state = state;
                    let _ = lua_status_gui_tx.try_send(LuaScriptStatusUpdate {
                        status: map_script_state(state),
                        script_path: Some(path.clone()),
                    });
                    let mut last = last_script_path.lock().unwrap();
                    *last = path;
                }
                EngineCommand::PauseScript => {
                    let state = {
                        let mut lua = lua_iface.lock().unwrap();
                        lua.pause_script()
                    };
                    last_script_state = state;
                    let _ = lua_status_gui_tx.try_send(LuaScriptStatusUpdate {
                        status: map_script_state(state),
                        script_path: None,
                    });
                }
                EngineCommand::ResumeScript => {
                    let state = {
                        let mut lua = lua_iface.lock().unwrap();
                        lua.resume_script()
                    };
                    last_script_state = state;
                    let _ = lua_status_gui_tx.try_send(LuaScriptStatusUpdate {
                        status: map_script_state(state),
                        script_path: None,
                    });
                }
            }
        }

        // Call Lua process()
        let (draw_cmds, script_state) = {
            let mut lua = lua_iface.lock().unwrap();
            let state = lua.call_process();
            (lua.take_draw_commands(), state)
        };

        if script_state != last_script_state {
            last_script_state = script_state;
            let _ = lua_status_gui_tx.try_send(LuaScriptStatusUpdate {
                status: map_script_state(script_state),
                script_path: None,
            });
        }

        // Send draw commands to GUI
        if !draw_cmds.is_empty() {
            // Convert draw commands to serializable format
            let gui_cmds: Vec<LuaDrawCmd> = draw_cmds
                .iter()
                .filter_map(|cmd| {
                    // Serialize/deserialize through serde_json for compatibility
                    if let Ok(json) = serde_json::to_value(cmd) {
                        serde_json::from_value(json).ok()
                    } else {
                        None
                    }
                })
                .collect();
            let _ = lua_draw_gui_tx.try_send(gui_cmds);
        } else {
            let _ = lua_draw_gui_tx.try_send(Vec::new());
        }

        // Prepare radio frame
        {
            let mut r = radio.lock().unwrap();
            r.prepare_frame();
        }

        // Radio snapshot at frame end (before send) for logging.
        let log_command_map = {
            let r = match radio.lock() {
                Ok(guard) => guard,
                Err(poisoned) => {
                    warn!("Radio lock poisoned, recovering");
                    poisoned.into_inner()
                }
            };
            r.get_command_map().clone()
        };

        {
            let mut l = match logger.lock() {
                Ok(guard) => guard,
                Err(poisoned) => {
                    warn!("Logger lock poisoned, recovering");
                    poisoned.into_inner()
                }
            };
            l.log_frame(&log_blue_robots, &log_yellow_robots, &log_ball, &log_command_map);
        }

        // Send radio commands
        {
            let mut r = match radio.lock() {
                Ok(guard) => guard,
                Err(poisoned) => {
                    warn!("Radio lock poisoned, recovering");
                    poisoned.into_inner()
                }
            };
            let mut w = match world.write() {
                Ok(guard) => guard,
                Err(poisoned) => {
                    warn!("World lock poisoned, recovering");
                    poisoned.into_inner()
                }
            };
            r.send_commands(&mut w);

            if r.has_simulator() {
                r.step_simulator(&mut w, TICK.as_secs_f64());
                let _ = vision_gui_tx.try_send(vision::gui_update(&w, SIM_FRAME_RATE));
            }
        }

        // Sleep for remaining frame time, or just yield when running flat out
        match frame_duration(simulator) {
            Some(frame_duration) => {
                let elapsed = frame_start.elapsed();
                if elapsed < frame_duration {
                    tokio::time::sleep(frame_duration - elapsed).await;
                }
            }
            None => tokio::task::yield_now().await,
        }
    }
}
