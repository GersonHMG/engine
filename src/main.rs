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

#[tokio::main]
async fn main() {
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

    // Spawn Vision receiver task
    let world_for_vision = Arc::clone(&world);
    let _vision_handle = tokio::spawn(async move {
        if let Err(e) = vision::run_vision(&vision_ip, vision_port, world_for_vision).await {
            warn!("Vision task error: {e}");
        }
    });

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

        // 2. Send radio commands
        {
            let mut r = radio.lock().unwrap();
            r.send_commands();
        }

        // Sleep for remaining frame time
        let elapsed = frame_start.elapsed();
        if elapsed < frame_duration {
            tokio::time::sleep(frame_duration - elapsed).await;
        }
    }
}
