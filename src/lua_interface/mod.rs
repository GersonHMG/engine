// lua_interface/mod.rs — Lua scripting interface

pub(crate) mod api;

use crate::game_controller::GameState;
use crate::sender::radio::Radio;
use crate::types::{DrawCommand, MotionCommand};
use crate::world::World;
use mlua::prelude::*;
use mlua::Variadic;
use tokio::sync::mpsc;
use std::sync::{Arc, Mutex, RwLock};
use tracing::{error, info};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptExecState {
    NoScript,
    Running,
    Paused,
    Failed,
}

pub struct LuaInterface {
    lua: Lua,
    radio: Arc<Mutex<Radio>>,
    world: Arc<RwLock<World>>,
    game_state: Arc<Mutex<GameState>>,
    draw_commands: Arc<Mutex<Vec<DrawCommand>>>,
    log_tx: Option<mpsc::Sender<String>>,
    have_script: bool,
    is_paused: bool,
    has_failed: bool,
}

impl LuaInterface {
    pub fn new(
        radio: Arc<Mutex<Radio>>,
        world: Arc<RwLock<World>>,
        game_state: Arc<Mutex<GameState>>,
        log_tx: Option<mpsc::Sender<String>>,
    ) -> Self {
        let lua = Lua::new();

        let mut interface = Self {
            lua,
            radio,
            world,
            game_state,
            draw_commands: Arc::new(Mutex::new(Vec::new())),
            log_tx,
            have_script: false,
            is_paused: false,
            has_failed: false,
        };

        interface.register_functions();
        interface
    }

    fn register_functions(&mut self) {
        api::register_api_functions(
            &self.lua,
            Arc::clone(&self.radio),
            Arc::clone(&self.world),
            Arc::clone(&self.game_state),
            Arc::clone(&self.draw_commands),
        );
        self.register_print();
    }

    fn register_print(&mut self) {
        let log_tx = self.log_tx.clone();
        let print_fn = self.lua.create_function(move |_, args: Variadic<LuaValue>| {
            let mut parts = Vec::with_capacity(args.len());
            for value in args {
                parts.push(lua_value_to_string(&value));
            }

            if let Some(tx) = &log_tx {
                let line = parts.join(" ");
                let _ = tx.try_send(format!("[Lua][print] {line}"));
            }
            Ok(())
        });

        match print_fn {
            Ok(func) => {
                let _ = self.lua.globals().set("print", func);
            }
            Err(e) => {
                self.log_error(format!("Failed to register print(): {e}"));
            }
        }
    }

    fn log_info(&self, message: String) {
        info!("[Lua] {message}");
        if let Some(tx) = &self.log_tx {
            let _ = tx.try_send(format!("[Lua][info] {message}"));
        }
    }

    fn log_error(&self, message: String) {
        error!("[Lua] {message}");
        if let Some(tx) = &self.log_tx {
            let _ = tx.try_send(format!("[Lua][error] {message}"));
        }
    }

    pub fn run_script(&mut self, script_path: &str) -> ScriptExecState {
        // Keep newly loaded scripts paused until the user explicitly resumes.
        self.is_paused = true;
        self.has_failed = false;

        // Reinitialize Lua state
        self.lua = Lua::new();
        self.register_functions();

        // Set package.path to include the script's directory
        if let Some(dir) = std::path::Path::new(script_path).parent() {
            let dir_str = dir.to_string_lossy().replace('\\', "/");
            let path_cmd = format!("package.path = '{}/?.lua;' .. package.path", dir_str);
            if let Err(e) = self.lua.load(&path_cmd).exec() {
                self.log_error(format!("Failed to set package path: {e}"));
            }
        }

        self.log_info(format!("Loading script: {script_path}"));

        match self.lua.load(std::path::Path::new(script_path)).exec() {
            Ok(()) => {
                self.log_info("Script loaded successfully (paused).".to_string());
                self.have_script = true;
            }
            Err(e) => {
                self.log_error(format!("Error loading script: {e}"));
                self.have_script = false;
                self.has_failed = true;
            }
        }

        self.script_state()
    }

    pub fn call_process(&mut self) -> ScriptExecState {
        if !self.have_script {
            return self.script_state();
        }

        if !self.is_paused {
            let globals = self.lua.globals();
            match globals.get::<LuaFunction>("process") {
                Ok(process) => match process.call::<()>(()) {
                    Ok(()) => {}
                    Err(e) => {
                        self.log_error(format!("Runtime error in process(): {e}"));
                        self.is_paused = true;
                        self.has_failed = true;
                    }
                },
                Err(_) => {
                    self.log_error("Error: process() is not defined in script!".to_string());
                    self.is_paused = true;
                    self.has_failed = true;
                }
            }
        } else {
            // When paused, send zero velocity to all robots
            if let Ok(mut r) = self.radio.lock() {
                for team in 0..2 {
                    for id in 0..6 {
                        let mut cmd = MotionCommand::new(id, team, 0.0, 0.0);
                        cmd.angular = Some(0.0);
                        r.add_motion_command(cmd);
                    }
                }
            }
        }

        self.script_state()
    }

    pub fn pause_script(&mut self) -> ScriptExecState {
        if !self.have_script {
            return self.script_state();
        }

        self.is_paused = true;
        self.has_failed = false;
        if let Ok(mut r) = self.radio.lock() {
            let cmd = MotionCommand::new(0, 0, 0.0, 0.0);
            r.add_motion_command(cmd);
        }
        self.log_info("Script paused.".to_string());

        self.script_state()
    }

    pub fn resume_script(&mut self) -> ScriptExecState {
        if !self.have_script {
            return self.script_state();
        }

        self.is_paused = false;
        self.has_failed = false;

        self.log_info("Script resumed.".to_string());

        self.script_state()
    }

    pub fn script_state(&self) -> ScriptExecState {
        if self.has_failed {
            ScriptExecState::Failed
        } else if !self.have_script {
            ScriptExecState::NoScript
        } else if self.is_paused {
            ScriptExecState::Paused
        } else {
            ScriptExecState::Running
        }
    }

    /// Take all queued draw commands (drains the buffer).
    pub fn take_draw_commands(&self) -> Vec<DrawCommand> {
        if let Ok(mut cmds) = self.draw_commands.lock() {
            std::mem::take(&mut *cmds)
        } else {
            Vec::new()
        }
    }
}

fn lua_value_to_string(value: &LuaValue) -> String {
    match value {
        LuaValue::Nil => "nil".to_string(),
        LuaValue::Boolean(v) => v.to_string(),
        LuaValue::Integer(v) => v.to_string(),
        LuaValue::Number(v) => v.to_string(),
        LuaValue::String(v) => v.to_string_lossy().to_string(),
        LuaValue::Table(_) => "{table}".to_string(),
        LuaValue::Function(_) => "{function}".to_string(),
        LuaValue::Thread(_) => "{thread}".to_string(),
        LuaValue::UserData(_) => "{userdata}".to_string(),
        LuaValue::LightUserData(_) => "{lightuserdata}".to_string(),
        LuaValue::Error(e) => format!("{e}"),
        LuaValue::Other(_) => "{other}".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex, RwLock};
    use crate::sender::radio::Radio;
    use crate::world::World;
    use crate::game_controller::GameState;

    #[test]
    fn test_trajectory_suite_loading() {
        let radio = Arc::new(Mutex::new(Radio::new(false, "", 115200)));
        let world = Arc::new(RwLock::new(World::new(6, 6, 12.0, 9.0)));
        let game_state = Arc::new(Mutex::new(GameState::new()));
        
        let mut interface = LuaInterface::new(radio, world, game_state, None);
        let state = interface.run_script("lua/run_trajectory_tests.lua");
        
        assert_eq!(state, ScriptExecState::Paused);
        assert!(!interface.has_failed);
        assert!(interface.have_script);

        // Resume and tick once to ensure it starts executing
        interface.resume_script();
        let state2 = interface.call_process();
        assert_eq!(state2, ScriptExecState::Running);
        assert!(!interface.has_failed);
    }

    #[test]
    fn test_lateral_movement_logging() {
        use crate::types::Vec2D;
        
        let radio = Arc::new(Mutex::new(Radio::new(false, "", 115200)));
        let world = Arc::new(RwLock::new(World::new(6, 6, 12.0, 9.0)));
        let game_state = Arc::new(Mutex::new(GameState::new()));
        
        // Add active robot
        {
            let mut w = world.write().unwrap();
            w.update_robot(0, 0, Vec2D::new(0.0, 0.0), 0.0, Vec2D::new(0.0, 0.0), 0.0);
        }
        
        let mut interface = LuaInterface::new(radio, world, game_state, None);
        let state = interface.run_script("lua/run_lateral_test.lua");
        
        assert_eq!(state, ScriptExecState::Paused);
        assert!(!interface.has_failed);
        
        interface.resume_script();
        
        let mut dir_path = String::new();
        // Run loop simulating 360 ticks (6 seconds)
        for tick in 0..360 {
            let elapsed = tick as f64 * (1.0 / 60.0);
            {
                let mut reg = crate::logger::get_registry().lock().unwrap();
                reg.set_current_elapsed(elapsed);
                if reg.session_dir.is_some() && dir_path.is_empty() {
                    dir_path = reg.session_dir.clone().unwrap();
                }
            }
            let state = interface.call_process();
            assert_eq!(state, ScriptExecState::Running);
            assert!(!interface.has_failed);
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        
        assert!(!dir_path.is_empty(), "Session dir should have been active during the test");
        
        // Stop session (it is already closed by the script, but stop_session is safe/idempotent)
        {
            let mut reg = crate::logger::get_registry().lock().unwrap();
            reg.stop_session();
        }
        
        // Verify files exist and are not empty
        let csv_path = format!("{}/lateral_movement_test.csv", dir_path);
        let json_path = format!("{}/lateral_movement_test.json", dir_path);
        
        assert!(std::path::Path::new(&csv_path).exists(), "CSV log file does not exist at {}", csv_path);
        assert!(std::path::Path::new(&json_path).exists(), "JSON report file does not exist at {}", json_path);
        
        let csv_len = std::fs::metadata(&csv_path).unwrap().len();
        let json_len = std::fs::metadata(&json_path).unwrap().len();
        
        assert!(csv_len > 100, "CSV file is empty or too small: {} bytes", csv_len);
        assert!(json_len > 50, "JSON file is empty or too small: {} bytes", json_len);
        
        // Cleanup logs
        let _ = std::fs::remove_file(&csv_path);
        let _ = std::fs::remove_file(&json_path);
        let _ = std::fs::remove_dir(&dir_path);
    }
}