// lua_interface/mod.rs — Lua scripting interface

mod api;

use crate::game_controller::GameState;
use crate::sender::radio::Radio;
use crate::types::{DrawCommand, MotionCommand};
use crate::world::World;
use mlua::prelude::*;
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
    have_script: bool,
    is_paused: bool,
    has_failed: bool,
}

impl LuaInterface {
    pub fn new(
        radio: Arc<Mutex<Radio>>,
        world: Arc<RwLock<World>>,
        game_state: Arc<Mutex<GameState>>,
    ) -> Self {
        let lua = Lua::new();

        let mut interface = Self {
            lua,
            radio,
            world,
            game_state,
            draw_commands: Arc::new(Mutex::new(Vec::new())),
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
                error!("[Lua] Failed to set package path: {e}");
            }
        }

        info!("[Lua] Loading script: {script_path}");

        match self.lua.load(std::path::Path::new(script_path)).exec() {
            Ok(()) => {
                info!("[Lua] Script loaded successfully (paused).");
                self.have_script = true;
            }
            Err(e) => {
                error!("[Lua] Error loading script: {e}");
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
                        error!("[Lua] Runtime error in process(): {e}");
                        self.is_paused = true;
                        self.has_failed = true;
                    }
                },
                Err(_) => {
                    error!("[Lua] Error: process() is not defined in script!");
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
        info!("[Lua] Script paused.");

        self.script_state()
    }

    pub fn resume_script(&mut self) -> ScriptExecState {
        if !self.have_script {
            return self.script_state();
        }

        self.is_paused = false;
        self.has_failed = false;

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