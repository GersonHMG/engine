// Sysmic Robotics — RoboCup SSL Engine (Rust)
// Entry point — Iced GUI with async engine loop

mod types;
mod proto;
mod vision;
mod tracker;
mod game_controller;
mod world;
mod environment;
mod motion;
mod packet_serializer;
mod grsim;
mod radio;
mod lua_interface;
mod console;
mod logger;
mod gui;

use std::sync::{Arc, RwLock, Mutex};
use std::time::{Duration, Instant};
use serde::Deserialize;

use tracing::{info, warn};

use crate::game_controller::GameState;
use crate::lua_interface::LuaInterface;
use crate::radio::Radio;
use crate::world::World;
use crate::logger::Logger;
use crate::types::MotionCommand;
use crate::gui::{EngineApp, EngineCommand, GuiChannels, LuaDrawCmd, VisionUpdate};

#[derive(Deserialize, Clone, Debug)]
pub struct ControllerParams {
    pub lat_kp: f64,
    pub lat_ki: f64,
    pub lat_kd: f64,
    pub speed_kp: f64,
    pub head_kp: f64,
    pub target_speed: f64,
    pub lookahead: f64,
    pub bangbang_a_max: f64,
    pub bangbang_v_max: f64,
    pub pid_kp: f64,
    pub pid_max_v: f64,
}

#[derive(Default)]
struct VisionState {
    handle: Option<tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>>>,
    tx: Option<tokio::sync::mpsc::Sender<vision::VisionCommand>>,
    ip: String,
    port: u16,
}

fn main() -> iced::Result {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Create channels between GUI and engine
    let (vision_tx, vision_rx) = tokio::sync::mpsc::channel::<VisionUpdate>(256);
    let (lua_draw_tx, lua_draw_rx) = tokio::sync::mpsc::channel::<Vec<LuaDrawCmd>>(256);
    let (command_tx, command_rx) = tokio::sync::mpsc::channel::<EngineCommand>(256);

    let gui_channels = GuiChannels {
        vision_rx,
        lua_draw_rx,
        command_tx: command_tx.clone(),
    };

    // Spawn the engine in a background tokio runtime
    let command_rx = Arc::new(Mutex::new(Some(command_rx)));
    let vision_tx_clone = vision_tx.clone();
    let lua_draw_tx_clone = lua_draw_tx.clone();
    let command_rx_clone = command_rx.clone();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
        rt.block_on(async move {
            let rx = command_rx_clone.lock().unwrap().take().expect("command_rx already taken");
            run_engine(vision_tx_clone, lua_draw_tx_clone, rx).await;
        });
    });

    // Wrap gui_channels so the boot closure can be Fn (not FnOnce)
    let gui_channels = std::sync::Arc::new(std::sync::Mutex::new(Some(gui_channels)));

    // Run Iced daemon on the main thread (multi-window)
    iced::daemon(
        move || {
            let channels = gui_channels.lock().unwrap().take()
                .expect("GUI channels already consumed");
            EngineApp::boot(channels)
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
    mut command_rx: tokio::sync::mpsc::Receiver<EngineCommand>,
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
    let world = Arc::new(RwLock::new(World::new(blue_team_size, yellow_team_size)));
    let game_state = Arc::new(Mutex::new(GameState::new()));
    let radio = Arc::new(Mutex::new(Radio::new(use_radio, &radio_port, radio_baud)));
    let vision_state = Arc::new(Mutex::new(VisionState::default()));
    let logger = Arc::new(Mutex::new(Logger::new(Arc::clone(&world), Arc::clone(&radio))));

    let lua_iface = Arc::new(Mutex::new(LuaInterface::new(
        Arc::clone(&radio),
        Arc::clone(&world),
        Arc::clone(&game_state),
    )));

    let last_script_path = Arc::new(Mutex::new(String::new()));

    // Spawn Vision receiver task
    {
        let mut vs = vision_state.lock().unwrap();
        vs.ip = vision_ip.clone();
        vs.port = vision_port;

        let (tx, rx) = tokio::sync::mpsc::channel(32);
        vs.tx = Some(tx);

        let world_for_vision = Arc::clone(&world);
        let ip_clone = vision_ip.clone();
        let gui_tx = vision_gui_tx.clone();

        let handle = tokio::spawn(async move {
            if let Err(e) = vision::run_vision(ip_clone, vision_port, world_for_vision, gui_tx, rx).await {
                warn!("Vision task error: {e}");
                Ok(())
            } else {
                Ok(())
            }
        });
        vs.handle = Some(handle);
    }

    // Spawn Game Controller receiver task
    let game_state_for_ref = Arc::clone(&game_state);
    let _ref_handle = tokio::spawn(async move {
        if let Err(e) = game_controller::run_game_controller("224.5.23.1", 10003, game_state_for_ref).await {
            warn!("GameController task error: {e}");
        }
    });

    // Spawn Console reader
    let lua_for_console = Arc::clone(&lua_iface);
    let _console_handle = tokio::task::spawn_blocking(move || {
        console::run_console(lua_for_console);
    });

    // Run script from command line arg if provided
    if let Some(script_path) = std::env::args().nth(1) {
        let mut lua = lua_iface.lock().unwrap();
        lua.run_script(&script_path);
        let mut last = last_script_path.lock().unwrap();
        *last = script_path;
    }

    // Main update loop (~60 FPS)
    let frame_duration = Duration::from_micros(16_667);
    info!("Engine started. Running at ~60 FPS.");

    loop {
        let frame_start = Instant::now();

        // Process GUI commands
        while let Ok(cmd) = command_rx.try_recv() {
            match cmd {
                EngineCommand::UpdateVisionConnection { ip, port } => {
                    let mut vs = vision_state.lock().unwrap();
                    if vs.ip == ip && vs.port == port && vs.handle.is_some() {
                        continue;
                    }
                    vs.ip = ip.clone();
                    vs.port = port;
                    if let Some(handle) = vs.handle.take() {
                        handle.abort();
                    }
                    let (tx, rx) = tokio::sync::mpsc::channel(32);
                    vs.tx = Some(tx);
                    let world_clone = world.clone();
                    let ip_clone = ip.clone();
                    let gui_tx = vision_gui_tx.clone();
                    let handle = tokio::spawn(async move {
                        if let Err(e) = vision::run_vision(ip_clone, port, world_clone, gui_tx, rx).await {
                            warn!("Vision task error: {e}");
                            Ok(())
                        } else {
                            Ok(())
                        }
                    });
                    vs.handle = Some(handle);
                    info!("Restarted vision task with {}:{}", ip, port);
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
                    let cmd = MotionCommand { id, team, vx, vy, angular: omega };
                    r.add_motion_command(cmd);
                }
                EngineCommand::LoadScript { path } => {
                    let mut lua = lua_iface.lock().unwrap();
                    lua.run_script(&path);
                    let mut last = last_script_path.lock().unwrap();
                    *last = path;
                }
                EngineCommand::PauseScript => {
                    let mut lua = lua_iface.lock().unwrap();
                    lua.pause_script();
                }
                EngineCommand::ResumeScript => {
                    let mut lua = lua_iface.lock().unwrap();
                    lua.resume_script();
                }
            }
        }

        // Call Lua process()
        let draw_cmds = {
            let mut lua = lua_iface.lock().unwrap();
            lua.call_process();
            lua.take_draw_commands()
        };

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

        // Log Frame (CSV Recording)
        {
            let mut l = match logger.lock() {
                Ok(guard) => guard,
                Err(poisoned) => {
                    warn!("Logger lock poisoned, recovering");
                    poisoned.into_inner()
                }
            };
            l.log_frame();
        }

        // Prepare radio frame
        {
            let mut r = radio.lock().unwrap();
            r.prepare_frame();
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
        }

        // Sleep for remaining frame time
        let elapsed = frame_start.elapsed();
        if elapsed < frame_duration {
            tokio::time::sleep(frame_duration - elapsed).await;
        }
    }
}
