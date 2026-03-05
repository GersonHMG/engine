// Sysmic Robotics — RoboCup SSL Engine (Rust)
// Entry point

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

use std::path::PathBuf;
use std::sync::{Arc, RwLock, Mutex};
use std::time::{Duration, Instant};
use serde::Deserialize;

use tracing::{info, warn};
use tauri::Manager; // Fix: Import Manager trait

use crate::game_controller::GameState;
use crate::lua_interface::LuaInterface;
use crate::radio::Radio;
use crate::world::World;
use crate::logger::Logger;
use crate::types::{PathTestState, Vec2D, MotionCommand};

#[derive(Default)]
struct VisionState {
    handle: Option<tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>>>,
    tx: Option<tokio::sync::mpsc::Sender<vision::VisionCommand>>,
    ip: String,
    port: u16,
}

struct AppState {
    world: Arc<RwLock<World>>,
    vision_state: Arc<Mutex<VisionState>>,
    logger: Arc<Mutex<Logger>>,
    radio: Arc<Mutex<Radio>>, // Needed for Xbox commands
    path_test: Arc<Mutex<Option<PathTestState>>>,
    lua_interface: Arc<Mutex<LuaInterface>>,
    last_script_path: Arc<Mutex<String>>,
}

#[tauri::command]
async fn update_vision_connection(
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
    ip: String,
    port: u16,
) -> Result<(), String> {
    let mut vision_state = state.vision_state.lock().unwrap();

    // Check if changed
    if vision_state.ip == ip && vision_state.port == port && vision_state.handle.is_some() {
        return Ok(());
    }

    vision_state.ip = ip.clone();
    vision_state.port = port;

    // Abort existing task
    if let Some(handle) = vision_state.handle.take() {
        handle.abort();
    }

    // Spawn new task
    let (tx, rx) = tokio::sync::mpsc::channel(32);
    vision_state.tx = Some(tx);

    let world_clone = state.world.clone();
    let app_handle_clone = app_handle.clone();
    let ip_clone = ip.clone();

    let handle = tokio::spawn(async move {
        // Fix: Pass ip_clone directly (String), don't borrow
        if let Err(e) = vision::run_vision(ip_clone, port, world_clone, app_handle_clone, rx).await {
            warn!("Vision task error: {e}");
            Ok(()) 
        } else {
            Ok(())
        }
    });
    
    vision_state.handle = Some(handle);
    
    info!("Restarted vision task with {}:{}", ip, port);
    Ok(())
}

#[tauri::command]
async fn update_radio_config(
    state: tauri::State<'_, AppState>,
    use_radio: bool,
    port_name: String,
    baud_rate: u32,
) -> Result<(), String> {
    let mut radio = state.radio.lock().unwrap();
    radio.reconfigure(use_radio, &port_name, baud_rate);
    info!("Radio reconfigured: use_radio={}, port={}, baud={}", use_radio, port_name, baud_rate);
    Ok(())
}

#[tauri::command]
async fn update_tracker_config(
    state: tauri::State<'_, AppState>,
    enabled: bool,
    process_noise_p: f64,
    process_noise_v: f64,
    measurement_noise: f64,
) -> Result<(), String> {
    let tx = {
        let vision_state = state.vision_state.lock().unwrap();
        vision_state.tx.clone()
    };
    
    if let Some(tx) = tx {
        let cmd = vision::VisionCommand::UpdateTrackerConfig {
            enabled,
            process_noise_p,
            process_noise_v,
            measurement_noise, // Fixed typo from 'measurent_noise' if any
        };
        tx.send(cmd).await.map_err(|e| e.to_string())?;
    } else {
        return Err("Vision task not running".into());
    }
    Ok(())
}

#[tauri::command]
async fn start_recording(
    state: tauri::State<'_, AppState>,
    filename: String,
) -> Result<(), String> {
    let mut logger = state.logger.lock().unwrap();
    if logger.is_logging() {
       return Err("Already recording".into());
    }
    
    logger.start_logging(Some(&filename));
    info!("Started recording to {}", filename);
    Ok(())
}

#[tauri::command]
async fn stop_recording(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut logger = state.logger.lock().unwrap();
    if !logger.is_logging() {
        return Ok(());
    }
    
    logger.stop_logging();
    info!("Stopped recording");
    Ok(())
}

#[tauri::command]
async fn send_robot_command(
    state: tauri::State<'_, AppState>,
    id: i32,
    team: i32,
    vx: f64,
    vy: f64,
    omega: f64,
) -> Result<(), String> {
    // Clear any path test going on so manual control takes over
    if (vx.abs() > 0.01 || vy.abs() > 0.01 || omega.abs() > 0.01) {
        if let Ok(mut pt) = state.path_test.lock() {
            *pt = None;
        }
    }

    let mut radio = state.radio.lock().unwrap();
    // Use radio/grsim to send command
    // radio struct has add_motion_command
    use crate::types::MotionCommand;
    let cmd = MotionCommand {
        id,
        team,
        vx,
        vy,
        angular: omega,
    };
    radio.add_motion_command(cmd);
    Ok(())
}

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

#[derive(Deserialize)]
struct PointReq {
    x: f64,
    y: f64,
}

#[tauri::command]
async fn send_path_test(
    state: tauri::State<'_, AppState>,
    id: i32,
    team: i32,
    controller: String,
    params: ControllerParams,
    points: Vec<PointReq>,
) -> Result<(), String> {
    if points.is_empty() {
        return Ok(());
    }

    let vec2d_points = points.into_iter().map(|p| Vec2D::new(p.x, p.y)).collect();
    let path_state = PathTestState::new(id, team, controller, params, vec2d_points);

    if let Ok(mut pt) = state.path_test.lock() {
        *pt = Some(path_state);
        info!("Started PathTest for robot {} team {} with {} points", id, team, pt.as_ref().unwrap().points.len());
    }

    Ok(())
}

#[tauri::command]
async fn load_script(
    state: tauri::State<'_, AppState>,
    path: String,
) -> Result<(), String> {
    let mut lua = state.lua_interface.lock().map_err(|e| e.to_string())?;
    lua.run_script(&path);
    let mut last = state.last_script_path.lock().map_err(|e| e.to_string())?;
    *last = path;
    Ok(())
}

#[tauri::command]
async fn pause_script(
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let mut lua = state.lua_interface.lock().map_err(|e| e.to_string())?;
    lua.pause_script();
    Ok(())
}

#[tauri::command]
async fn resume_script(
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let mut lua = state.lua_interface.lock().map_err(|e| e.to_string())?;
    lua.resume_script();
    Ok(())
}

#[tauri::command]
async fn reload_script(
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let last = state.last_script_path.lock().map_err(|e| e.to_string())?.clone();
    if last.is_empty() {
        return Err("No script loaded yet".into());
    }
    let mut lua = state.lua_interface.lock().map_err(|e| e.to_string())?;
    lua.run_script(&last);
    Ok(())
}

#[tokio::main]
async fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let app_handle = app.handle();
            
            tokio::spawn(async move {
                run_engine(app_handle).await;
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            update_vision_connection, 
            update_tracker_config,
            update_radio_config,
            start_recording,
            stop_recording,
            send_robot_command,
            send_path_test,
            load_script,
            pause_script,
            resume_script,
            reload_script
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

async fn run_engine(app_handle: tauri::AppHandle) {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Configuration Defaults (No config.ini)
    
    // Vision
    let vision_ip = "224.5.23.2".to_string();
    let vision_port = 10020u16;

    // World
    let blue_team_size = 6;
    let yellow_team_size = 6;

    // Radio
    let use_radio = false; // Default to simulator
    let radio_port = "/dev/ttyUSB0".to_string();
    let radio_baud = 115200;

    // Shared state

    // Shared state
    let world = Arc::new(RwLock::new(World::new(blue_team_size, yellow_team_size)));
    let game_state = Arc::new(Mutex::new(GameState::new()));
    let radio = Arc::new(Mutex::new(Radio::new(use_radio, &radio_port, radio_baud)));
    
    let vision_state = Arc::new(Mutex::new(VisionState::default()));
    
    // Initialize Logger
    let logger = Arc::new(Mutex::new(Logger::new(Arc::clone(&world), Arc::clone(&radio))));

    let path_test = Arc::new(Mutex::new(None));

    // Create Lua interface
    let lua_iface = Arc::new(Mutex::new(LuaInterface::new(
        Arc::clone(&radio),
        Arc::clone(&world),
        Arc::clone(&game_state),
    )));

    // Register state
    let last_script_path = Arc::new(Mutex::new(String::new()));

    app_handle.manage(AppState {
        world: world.clone(),
        vision_state: vision_state.clone(),
        logger: logger.clone(),
        radio: radio.clone(),
        path_test: path_test.clone(),
        lua_interface: lua_iface.clone(),
        last_script_path: last_script_path.clone(),
    });

    // Spawn Vision receiver task
    {
        // Initial spawn using the helper command logic or manually
        let mut vs = vision_state.lock().unwrap();
        vs.ip = vision_ip.clone();
        vs.port = vision_port;
        
        let (tx, rx) = tokio::sync::mpsc::channel(32);
        vs.tx = Some(tx);
        
        let world_for_vision = Arc::clone(&world);
        let app_handle_vision = app_handle.clone();
        let ip_clone = vision_ip.clone();
        
        let handle = tokio::spawn(async move {
            // Fix: Pass ip_clone directly (String), don't borrow
            if let Err(e) = vision::run_vision(ip_clone, vision_port, world_for_vision, app_handle_vision, rx).await {
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
        if let Err(e) =
            game_controller::run_game_controller("224.5.23.1", 10003, game_state_for_ref).await
        {
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
    let frame_duration = Duration::from_micros(16_667); // ~60 Hz
    info!("Engine started. Running at ~60 FPS.");
    
    loop {
        let frame_start = Instant::now();

        // 1. Call Lua process()
        {
            let mut lua = lua_iface.lock().unwrap();
            lua.call_process();
        }

        // 3. Log Frame (CSV Recording)
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

        // 3. Prepare radio frame (zero-velocity defaults for active robots)
        {
            let mut r = radio.lock().unwrap();
            r.prepare_frame();
        }

        // 5. Send radio commands
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
