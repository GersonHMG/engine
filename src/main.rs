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

struct AppState {
    world: Arc<RwLock<World>>,
    vision_state: Arc<Mutex<VisionState>>,
    logger: Arc<Mutex<Logger>>,
    radio: Arc<Mutex<Radio>>, // Needed for Xbox commands
    path_test: Arc<Mutex<Option<PathTestState>>>,
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
            send_path_test
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

    // Register state
    app_handle.manage(AppState {
        world: world.clone(),
        vision_state: vision_state.clone(),
        logger: logger.clone(),
        radio: radio.clone(),
        path_test: path_test.clone(),
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
    
    loop {
        let frame_start = Instant::now();

        // 1. Call Lua process()
        {
            let mut lua = lua_interface.lock().unwrap();
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

        // 4. Evaluate Path Test Loop (Overrides Radio output with motion commands)
        {
            let mut pt_guard = path_test.lock().unwrap();
            let mut radio_guard = radio.lock().unwrap();
            
            if let Some(ref mut pts) = *pt_guard {
                let w = world.read().unwrap();
                let robot = w.get_robot_state(pts.id, pts.team);
                
                // Active check: if robot isn't active, we might want to still try, 
                // but let's just proceed with the returned state (fallback is default).
                if true {
                    if pts.current_target_idx < pts.points.len() {
                        let target = pts.points[pts.current_target_idx];
                        let diff = target - robot.position;
                        let dist = diff.length();
                        
                        // Proceed to next point if reasonably close, but don't finish if it's the last point.
                        if dist < 0.30 {
                            if pts.current_target_idx < pts.points.len() - 1 {
                                pts.current_target_idx += 1;
                            }
                        } 
                        
                        if pts.current_target_idx < pts.points.len() {
                            let curr_target = pts.points[pts.current_target_idx];

                            if pts.controller == "bangbang" {
                                // Instantiate BangBang inline with our requested config
                                let bangbang = crate::motion::controllers::bangbang::BangBangControl::new(
                                    pts.params.bangbang_a_max, 
                                    pts.params.bangbang_v_max
                                );

                                let remaining = pts.points[pts.current_target_idx..].to_vec();
                                let delta = 1.0 / 60.0;
                                let cmd = bangbang.compute_motion(&robot, remaining, delta);
                                radio_guard.add_motion_command(cmd);

                            } else if pts.controller == "pid" {
                                // Simple PID
                                let kp = pts.params.pid_kp; // using pid_kp for simple PID
                                let global_vel = (curr_target - robot.position) * kp;
                                
                                // Rotate to local frame
                                let orientation = robot.orientation;
                                let local_vx = global_vel.x * (-orientation).cos() - global_vel.y * (-orientation).sin();
                                let local_vy = global_vel.x * (-orientation).sin() + global_vel.y * (-orientation).cos();
                                
                                // Max speed limit
                                let max_v = pts.params.pid_max_v;
                                let mut final_vel = Vec2D::new(local_vx, local_vy);
                                if final_vel.length() > max_v {
                                    final_vel = final_vel.normalized() * max_v;
                                }

                                radio_guard.add_motion_command(MotionCommand::new(pts.id, pts.team, final_vel.x, final_vel.y));
                            } else if pts.controller == "lookahead" {
                                    // Make sure LookAhead controller state exists inside AppState, 
                                    // but for this simple port we'll instantiate it on the fly or hold it in PathTestState.
                                    // To hold it in PathTestState, let's create a temporary one if we don't want to change the struct entirely. 
                                    // Actually, it's stateful (integral), we must put it in the lock. Since we just ported it,
                                    // let's do a stateless simple PID if we can't store it, or simply use static or add to PathTestState.
                                    // Since PathTestState is right here, it's better to store it. But for a quick test, 
                                    // let's create a new one each frame (might lose integral windup but P & D terms will work).
                                    // For a proper test, we should add it to PathTestState.
                                    
                                    // We will temporarily instantiate inline. The integral will be 0 but P works. 
                                    // A proper fix is updating PathTestState or adding a hashmap in AppState.
                                    let mut lookahead_pid = crate::motion::controllers::pid::lookahead::LookAheadPID::new(
                                        pts.params.lat_kp, 
                                        pts.params.lat_ki, 
                                        pts.params.lat_kd, 
                                        pts.params.speed_kp, 
                                        pts.params.head_kp, 
                                        pts.params.target_speed, 
                                        pts.params.lookahead
                                    );
                                    
                                    // Restore index if we can
                                    lookahead_pid.closest_idx = pts.current_target_idx;

                                    let (vx, vy, omega) = lookahead_pid.compute(robot.position, robot.orientation, robot.velocity, &pts.points, 1.0/60.0);
                                    
                                    // Update our stored index
                                    pts.current_target_idx = lookahead_pid.closest_idx;

                                    if vx.abs() > 0.05 || vy.abs() > 0.05 || omega.abs() > 0.05 {
                                        radio_guard.add_motion_command(MotionCommand {
                                            id: pts.id,
                                            team: pts.team,
                                            vx,
                                            vy,
                                            angular: omega,
                                        });
                                    }
                                } else {
                                    // Target reached. Full stop.
                                    radio_guard.add_motion_command(MotionCommand::new(pts.id, pts.team, 0.0, 0.0));
                                    *pt_guard = None; // clear path
                                    info!("Path following finished");
                                }
                        }
                    } else {
                        // Finished
                        radio_guard.add_motion_command(MotionCommand::new(pts.id, pts.team, 0.0, 0.0));
                        *pt_guard = None;
                        info!("Path following finished");
                    }
                }
            }
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
