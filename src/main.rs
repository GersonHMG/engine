// Sysmic Robotics — RoboCup SSL Engine (Rust)
// Entry point

mod types;
mod proto;
mod vision;
mod tracker;
mod game_controller;
mod world;
mod pid;
mod trajectory;
mod environment;
mod path_planner;
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

use tracing::{info, warn};
use tauri::Manager; // Fix: Import Manager trait

use crate::game_controller::GameState;
use crate::lua_interface::LuaInterface;
use crate::radio::Radio;
use crate::world::World;

/// Load config.ini and return parsed settings as an `ini::Ini`.
fn load_config() -> ini::Ini {
    // Look for config.ini next to the executable, then fall back to cwd
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));

    let config_path = exe_dir.join("config.ini");
    if config_path.exists() {
        ini::Ini::load_from_file(&config_path)
            .unwrap_or_else(|e| panic!("Failed to load {}: {e}", config_path.display()))
    } else {
        // Try current working directory
        let cwd_path = PathBuf::from("config.ini");
        ini::Ini::load_from_file(&cwd_path)
            .unwrap_or_else(|e| panic!("Failed to load config.ini: {e}"))
    }
}

fn get_str(ini: &ini::Ini, section: &str, key: &str, default: &str) -> String {
    ini.get_from(Some(section), key)
        .unwrap_or(default)
        .to_string()
}

fn get_int(ini: &ini::Ini, section: &str, key: &str, default: i64) -> i64 {
    ini.get_from(Some(section), key)
        .and_then(|v: &str| v.parse::<i64>().ok())
        .unwrap_or(default)
}

fn get_bool(ini: &ini::Ini, section: &str, key: &str, default: bool) -> bool {
    ini.get_from(Some(section), key)
        .map(|v: &str| v.eq_ignore_ascii_case("true") || v == "1")
        .unwrap_or(default)
}

#[derive(Default)]
struct VisionState {
    handle: Option<tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>>>,
    tx: Option<tokio::sync::mpsc::Sender<vision::VisionCommand>>,
    ip: String,
    port: u16,
}

#[derive(Default)]
// --- Recording Structs ---
struct RecordingState {
    writer: Option<csv::Writer<std::fs::File>>,
    active: bool,
}

struct AppState {
    world: Arc<RwLock<World>>,
    vision_state: Arc<Mutex<VisionState>>,
    recording_state: Arc<Mutex<RecordingState>>,
    radio: Arc<Mutex<Radio>>, // Needed for Xbox commands
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
    let mut rec = state.recording_state.lock().unwrap();
    if rec.active {
        return Err("Already recording".into());
    }

    match csv::Writer::from_path(&filename) {
        Ok(w) => {
            rec.writer = Some(w);
            rec.active = true;
            info!("Started recording to {}", filename);
            Ok(())
        }
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
async fn stop_recording(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut rec = state.recording_state.lock().unwrap();
    if !rec.active {
        return Ok(());
    }
    
    if let Some(mut w) = rec.writer.take() {
        w.flush().map_err(|e| e.to_string())?;
    }
    rec.active = false;
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
            start_recording,
            stop_recording,
            send_robot_command
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

async fn run_engine(app_handle: tauri::AppHandle) {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let config = load_config();

    // Vision
    let vision_ip = get_str(&config, "Vision", "ip_address", "224.5.23.2");
    let vision_port = get_int(&config, "Vision", "port", 10020) as u16;

    // World
    let blue_team_size = get_int(&config, "World", "blue_team_size", 6) as i32;
    let yellow_team_size = get_int(&config, "World", "yellow_team_size", 6) as i32;

    // Performance
    let _update_fps = get_int(&config, "Performance", "update_fps", 60) as u32;

    // Radio
    let use_radio = get_bool(&config, "Radio", "use_radio", false);
    let radio_port = get_str(&config, "Radio", "port_name", "/dev/ttyUSB0");
    let radio_baud = get_int(&config, "Radio", "baud_rate", 115200) as u32;

    // Shared state
    let world = Arc::new(RwLock::new(World::new(blue_team_size, yellow_team_size)));
    let game_state = Arc::new(Mutex::new(GameState::new()));
    let radio = Arc::new(Mutex::new(Radio::new(use_radio, &radio_port, radio_baud)));
    
    let vision_state = Arc::new(Mutex::new(VisionState::default()));
    let recording_state = Arc::new(Mutex::new(RecordingState { writer: None, active: false }));

    // Register state
    app_handle.manage(AppState {
        world: world.clone(),
        vision_state: vision_state.clone(),
        recording_state: recording_state.clone(),
        radio: radio.clone(),
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
            // Fix: Pass vision_ip directly (String), don't borrow
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

    // Create Lua interface
    let lua_interface = Arc::new(Mutex::new(LuaInterface::new(
        Arc::clone(&radio),
        Arc::clone(&world),
        Arc::clone(&game_state),
    )));

    // Spawn Console reader
    let lua_for_console = Arc::clone(&lua_interface);
    let _console_handle = tokio::task::spawn_blocking(move || {
        console::run_console(lua_for_console);
    });

    // Run script from command line arg if provided
    if let Some(script_path) = std::env::args().nth(1) {
        let mut lua = lua_interface.lock().unwrap();
        lua.run_script(&script_path);
    }

    // Main update loop (~60 FPS)
    let frame_duration = Duration::from_micros(16_667); // ~60 Hz
    info!("Engine started. Running at ~60 FPS.");
    
    // Recording helper function
    #[derive(serde::Serialize)]
    struct CsvRow {
        timestamp: u128,
        ball_x: f64,
        ball_y: f64,
        // Can add more fields or just dump JSON string if needed, but CSV usually flat.
        // For now let's just dump ball pos.
    }

    loop {
        let frame_start = Instant::now();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();

        // 1. Call Lua process()
        {
            let mut lua = lua_interface.lock().unwrap();
            lua.call_process();
        }

        // 2. Send radio commands
        {
            let mut r = radio.lock().unwrap();
            r.send_commands();
        }
        
        // 3. Recording
        {
            let mut rec = recording_state.lock().unwrap();
            if rec.active {
                if let Some(w) = rec.writer.as_mut() {
                    let w_read = world.read().unwrap();
                     let row = CsvRow {
                        timestamp,
                        ball_x: w_read.ball.position.x,
                        ball_y: w_read.ball.position.y,
                    };
                    if let Err(e) = w.serialize(&row) {
                        warn!("Failed to write CSV row: {e}");
                    }
                }
            }
        }

        // Sleep for remaining frame time
        let elapsed = frame_start.elapsed();
        if elapsed < frame_duration {
            tokio::time::sleep(frame_duration - elapsed).await;
        }
    }
}
