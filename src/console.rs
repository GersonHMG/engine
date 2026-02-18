// console.rs — Console reader for interactive commands
// Port of consolereader/consolereader.cpp

use crate::lua_interface::LuaInterface;
use std::io::{self, BufRead, Write};
use std::sync::{Arc, Mutex};
use tracing::{debug, info, warn};

/// Run the console reader in a blocking thread. Reads stdin and dispatches commands.
pub fn run_console(lua_interface: Arc<Mutex<LuaInterface>>) {
    info!("[ConsoleReader] Started. Type 'run <path>', 'pause', 'resume', 'reload', or 'exit'");

    let stdin = io::stdin();
    let mut last_script_path = String::new();

    loop {
        print!("> ");
        let _ = io::stdout().flush();

        let mut line = String::new();
        if stdin.lock().read_line(&mut line).is_err() {
            break;
        }
        let line = line.trim();

        if line.is_empty() {
            continue;
        }

        if line.starts_with("run ") {
            let path = line[4..].trim();
            if !path.is_empty() {
                if let Ok(mut lua) = lua_interface.lock() {
                    lua.run_script(path);
                    last_script_path = path.to_string();
                    debug!("Running script: {path}");
                }
            } else {
                warn!("No path provided after 'run'");
            }
        } else if line == "pause" {
            if let Ok(mut lua) = lua_interface.lock() {
                lua.pause_script();
                debug!("Script paused");
            }
        } else if line == "resume" {
            if let Ok(mut lua) = lua_interface.lock() {
                lua.resume_script();
                debug!("Script resumed");
            }
        } else if line == "reload" {
            if !last_script_path.is_empty() {
                if let Ok(mut lua) = lua_interface.lock() {
                    lua.run_script(&last_script_path);
                    debug!("Reloaded script: {}", last_script_path);
                }
            } else {
                warn!("No script has been run yet to reload.");
            }
        } else if line == "exit" {
            info!("[ConsoleReader] Exiting.");
            break;
        } else {
            warn!("Unknown command: {line}");
        }
    }
}
