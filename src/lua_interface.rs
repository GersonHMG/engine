// lua_interface.rs — Lua scripting interface
// Port of luainterface/luainterface.cpp + luabindings.cpp

use crate::game_controller::GameState;
use crate::motion::Motion;
use crate::radio::Radio;
use crate::types::{KickerCommand, MotionCommand, Vec2D};
use crate::world::World;
use mlua::prelude::*;
use std::sync::{Arc, Mutex, RwLock};
use tracing::{error, info};

pub struct LuaInterface {
    lua: Lua,
    radio: Arc<Mutex<Radio>>,
    world: Arc<RwLock<World>>,
    game_state: Arc<Mutex<GameState>>,
    have_script: bool,
    is_paused: bool,
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
            have_script: false,
            is_paused: false,
        };

        interface.register_functions();
        interface
    }

    fn register_functions(&mut self) {
        // We need to pass Arc clones into closures for Lua functions
        let radio = Arc::clone(&self.radio);
        let world = Arc::clone(&self.world);
        let game_state = Arc::clone(&self.game_state);

        let globals = self.lua.globals();

        // ── get_robot_state(id, team) ──
        {
            let w = Arc::clone(&world);
            let f = self
                .lua
                .create_function(move |lua, (id, team): (i32, i32)| {
                    let w = w.read().map_err(|e| LuaError::external(format!("{e}")))?;
                    let rs = w.get_robot_state(id, team);
                    let t = lua.create_table()?;
                    t.set("id", rs.id)?;
                    t.set("team", rs.team)?;
                    t.set("x", rs.position.x)?;
                    t.set("y", rs.position.y)?;
                    t.set("vel_x", rs.velocity.x)?;
                    t.set("vel_y", rs.velocity.y)?;
                    t.set("orientation", rs.orientation)?;
                    t.set("omega", rs.angular_velocity)?;
                    t.set("active", rs.active)?;
                    Ok(t)
                })
                .unwrap();
            globals.set("get_robot_state", f).unwrap();
        }

        // ── get_ball_state() ──
        {
            let w = Arc::clone(&world);
            let f = self
                .lua
                .create_function(move |lua, ()| {
                    let w = w.read().map_err(|e| LuaError::external(format!("{e}")))?;
                    let bs = w.get_ball_state();
                    let t = lua.create_table()?;
                    t.set("x", bs.position.x)?;
                    t.set("y", bs.position.y)?;
                    t.set("vel_x", bs.velocity.x)?;
                    t.set("vel_y", bs.velocity.y)?;
                    Ok(t)
                })
                .unwrap();
            globals.set("get_ball_state", f).unwrap();
        }

        // ── get_blue_team_state() ──
        {
            let w = Arc::clone(&world);
            let f = self
                .lua
                .create_function(move |lua, ()| {
                    let w = w.read().map_err(|e| LuaError::external(format!("{e}")))?;
                    let team = w.get_blue_team_state();
                    let result = lua.create_table()?;
                    for (i, rs) in team.iter().enumerate() {
                        let t = lua.create_table()?;
                        t.set("id", rs.id)?;
                        t.set("team", 0)?;
                        t.set("x", rs.position.x)?;
                        t.set("y", rs.position.y)?;
                        t.set("vel_x", rs.velocity.x)?;
                        t.set("vel_y", rs.velocity.y)?;
                        t.set("orientation", rs.orientation)?;
                        t.set("omega", rs.angular_velocity)?;
                        t.set("active", rs.active)?;
                        result.set(i + 1, t)?; // Lua is 1-based
                    }
                    Ok(result)
                })
                .unwrap();
            globals.set("get_blue_team_state", f).unwrap();
        }

        // ── get_yellow_team_state() ──
        {
            let w = Arc::clone(&world);
            let f = self
                .lua
                .create_function(move |lua, ()| {
                    let w = w.read().map_err(|e| LuaError::external(format!("{e}")))?;
                    let team = w.get_yellow_team_state();
                    let result = lua.create_table()?;
                    for (i, rs) in team.iter().enumerate() {
                        let t = lua.create_table()?;
                        t.set("id", rs.id)?;
                        t.set("team", 1)?;
                        t.set("x", rs.position.x)?;
                        t.set("y", rs.position.y)?;
                        t.set("vel_x", rs.velocity.x)?;
                        t.set("vel_y", rs.velocity.y)?;
                        t.set("orientation", rs.orientation)?;
                        t.set("omega", rs.angular_velocity)?;
                        t.set("active", rs.active)?;
                        result.set(i + 1, t)?;
                    }
                    Ok(result)
                })
                .unwrap();
            globals.set("get_yellow_team_state", f).unwrap();
        }

        // ── get_ref_message() ──
        {
            let gs = Arc::clone(&game_state);
            let f = self
                .lua
                .create_function(move |_, ()| {
                    let gs = gs.lock().map_err(|e| LuaError::external(format!("{e}")))?;
                    Ok(gs.get_ref_message().to_string())
                })
                .unwrap();
            globals.set("get_ref_message", f).unwrap();
        }

        // ── send_velocity(id, team, vx, vy, omega) ──
        {
            let r = Arc::clone(&radio);
            let w = Arc::clone(&world);
            let f = self
                .lua
                .create_function(move |_, (id, team, vx, vy, omega): (i32, i32, f64, f64, f64)| {
                    let w = w.read().map_err(|e| LuaError::external(format!("{e}")))?;
                    let rs = w.get_robot_state(id, team);
                    if !rs.active {
                        return Ok(());
                    }
                    let mut cmd = MotionCommand::new(id, team, vx, vy);
                    cmd.angular = omega;
                    r.lock()
                        .map_err(|e| LuaError::external(format!("{e}")))?
                        .add_motion_command(cmd);
                    Ok(())
                })
                .unwrap();
            globals.set("send_velocity", f).unwrap();
        }

        // ── move_to(id, team, {x=, y=}) ──
        {
            let r = Arc::clone(&radio);
            let w = Arc::clone(&world);
            let f = self
                .lua
                .create_function(move |_, (id, team, point): (i32, i32, LuaTable)| {
                    let x: f64 = point.get("x")?;
                    let y: f64 = point.get("y")?;
                    let w = w.read().map_err(|e| LuaError::external(format!("{e}")))?;
                    let rs = w.get_robot_state(id, team);
                    if !rs.active {
                        return Ok(());
                    }
                    let motion = Motion::new();
                    let cmd = motion.move_to(&rs, Vec2D::new(x, y), &w);
                    r.lock()
                        .map_err(|e| LuaError::external(format!("{e}")))?
                        .add_motion_command(cmd);
                    Ok(())
                })
                .unwrap();
            globals.set("move_to", f).unwrap();
        }

        // ── move_direct(id, team, {x=, y=}) ──
        {
            let r = Arc::clone(&radio);
            let w = Arc::clone(&world);
            let f = self
                .lua
                .create_function(move |_, (id, team, point): (i32, i32, LuaTable)| {
                    let x: f64 = point.get("x")?;
                    let y: f64 = point.get("y")?;
                    let w = w.read().map_err(|e| LuaError::external(format!("{e}")))?;
                    let rs = w.get_robot_state(id, team);
                    if !rs.active {
                        return Ok(());
                    }
                    let motion = Motion::new();
                    let cmd = motion.move_direct(&rs, Vec2D::new(x, y));
                    r.lock()
                        .map_err(|e| LuaError::external(format!("{e}")))?
                        .add_motion_command(cmd);
                    Ok(())
                })
                .unwrap();
            globals.set("move_direct", f).unwrap();
        }

        // ── motion(id, team, {x=, y=}, kp_x?, ki_x?, kp_y?, ki_y?) ──
        {
            let r = Arc::clone(&radio);
            let w = Arc::clone(&world);
            let f = self
                .lua
                .create_function(
                    move |_,
                          (id, team, point, kp_x, ki_x, kp_y, ki_y): (
                        i32,
                        i32,
                        LuaTable,
                        Option<f64>,
                        Option<f64>,
                        Option<f64>,
                        Option<f64>,
                    )| {
                        let x: f64 = point.get("x")?;
                        let y: f64 = point.get("y")?;
                        let w = w.read().map_err(|e| LuaError::external(format!("{e}")))?;
                        let rs = w.get_robot_state(id, team);
                        if !rs.active {
                            return Ok(());
                        }
                        let motion = Motion::new();
                        let cmd = motion.motion(
                            &rs,
                            Vec2D::new(x, y),
                            &w,
                            kp_x.unwrap_or(0.5),
                            ki_x.unwrap_or(0.1),
                            kp_y.unwrap_or(0.5),
                            ki_y.unwrap_or(0.1),
                        );
                        r.lock()
                            .map_err(|e| LuaError::external(format!("{e}")))?
                            .add_motion_command(cmd);
                        Ok(())
                    },
                )
                .unwrap();
            globals.set("motion", f).unwrap();
        }

        // ── face_to(id, team, {x=, y=}, kp?, ki?, kd?) ──
        {
            let r = Arc::clone(&radio);
            let w = Arc::clone(&world);
            let f = self
                .lua
                .create_function(
                    move |_,
                          (id, team, point, kp, ki, kd): (
                        i32,
                        i32,
                        LuaTable,
                        Option<f64>,
                        Option<f64>,
                        Option<f64>,
                    )| {
                        let x: f64 = point.get("x")?;
                        let y: f64 = point.get("y")?;
                        let w = w.read().map_err(|e| LuaError::external(format!("{e}")))?;
                        let rs = w.get_robot_state(id, team);
                        if !rs.active {
                            return Ok(());
                        }
                        let motion = Motion::new();
                        let cmd = motion.face_to(
                            &rs,
                            Vec2D::new(x, y),
                            kp.unwrap_or(1.0),
                            ki.unwrap_or(1.0),
                            kd.unwrap_or(0.1),
                        );
                        r.lock()
                            .map_err(|e| LuaError::external(format!("{e}")))?
                            .add_motion_command(cmd);
                        Ok(())
                    },
                )
                .unwrap();
            globals.set("face_to", f).unwrap();
        }

        // ── kickx(id, team) ──
        {
            let r = Arc::clone(&radio);
            let f = self
                .lua
                .create_function(move |_, (id, team): (i32, i32)| {
                    let mut kicker = KickerCommand::new(id, team);
                    kicker.kick_x = true;
                    r.lock()
                        .map_err(|e| LuaError::external(format!("{e}")))?
                        .add_kicker_command(kicker);
                    Ok(())
                })
                .unwrap();
            globals.set("kickx", f).unwrap();
        }

        // ── kickz(id, team) ──
        {
            let r = Arc::clone(&radio);
            let f = self
                .lua
                .create_function(move |_, (id, team): (i32, i32)| {
                    let mut kicker = KickerCommand::new(id, team);
                    kicker.kick_z = true;
                    r.lock()
                        .map_err(|e| LuaError::external(format!("{e}")))?
                        .add_kicker_command(kicker);
                    Ok(())
                })
                .unwrap();
            globals.set("kickz", f).unwrap();
        }

        // ── dribbler(id, team, speed) ──
        {
            let r = Arc::clone(&radio);
            let f = self
                .lua
                .create_function(move |_, (id, team, speed): (i32, i32, f64)| {
                    let speed = speed.clamp(0.0, 10.0);
                    let mut kicker = KickerCommand::new(id, team);
                    kicker.dribbler = speed;
                    r.lock()
                        .map_err(|e| LuaError::external(format!("{e}")))?
                        .add_kicker_command(kicker);
                    Ok(())
                })
                .unwrap();
            globals.set("dribbler", f).unwrap();
        }

        // ── grsim.teleport_robot(id, team, x, y, dir) ──
        // ── grsim.teleport_ball(x, y) ──
        {
            let grsim_table = self.lua.create_table().unwrap();

            let r1 = Arc::clone(&radio);
            let teleport_robot = self
                .lua
                .create_function(move |_, (id, team, x, y, dir): (i32, i32, f64, f64, f64)| {
                    r1.lock()
                        .map_err(|e| LuaError::external(format!("{e}")))?
                        .teleport_robot(id, team, x, y, dir);
                    Ok(())
                })
                .unwrap();
            grsim_table.set("teleport_robot", teleport_robot).unwrap();

            let r2 = Arc::clone(&radio);
            let teleport_ball = self
                .lua
                .create_function(move |_, (x, y): (f64, f64)| {
                    r2.lock()
                        .map_err(|e| LuaError::external(format!("{e}")))?
                        .teleport_ball(x, y);
                    Ok(())
                })
                .unwrap();
            grsim_table.set("teleport_ball", teleport_ball).unwrap();

            globals.set("grsim", grsim_table).unwrap();
        }
    }

    pub fn run_script(&mut self, script_path: &str) {
        self.is_paused = false;

        // Reinitialize Lua state
        self.lua = Lua::new();
        self.register_functions();

        // Set package.path to include the script's directory
        if let Some(dir) = std::path::Path::new(script_path).parent() {
            let dir_str = dir.to_string_lossy().replace('\\', "/");
            let path_cmd = format!(
                "package.path = '{}/?.lua;' .. package.path",
                dir_str
            );
            if let Err(e) = self.lua.load(&path_cmd).exec() {
                error!("[Lua] Failed to set package path: {e}");
            }
        }

        info!("[Lua] Loading script: {script_path}");

        match self.lua.load(std::path::Path::new(script_path)).exec() {
            Ok(()) => {
                info!("[Lua] Script loaded successfully!");
                self.have_script = true;
            }
            Err(e) => {
                error!("[Lua] Error loading script: {e}");
                self.have_script = false;
            }
        }
    }

    pub fn call_process(&mut self) {
        if !self.have_script {
            return;
        }

        if !self.is_paused {
            let globals = self.lua.globals();
            match globals.get::<LuaFunction>("process") {
                Ok(process) => match process.call::<()>(()) {
                    Ok(()) => {}
                    Err(e) => {
                        error!("[Lua] Runtime error in process(): {e}");
                        self.is_paused = true;
                    }
                },
                Err(_) => {
                    error!("[Lua] Error: process() is not defined in script!");
                    self.is_paused = true;
                }
            }
        } else {
            // When paused, send zero velocity to all robots
            if let Ok(mut r) = self.radio.lock() {
                for team in 0..2 {
                    for id in 0..6 {
                        let mut cmd = MotionCommand::new(id, team, 0.0, 0.0);
                        cmd.angular = 0.0;
                        r.add_motion_command(cmd);
                    }
                }
            }
        }
    }

    pub fn pause_script(&mut self) {
        self.is_paused = true;
        if let Ok(mut r) = self.radio.lock() {
            let cmd = MotionCommand::new(0, 0, 0.0, 0.0);
            r.add_motion_command(cmd);
        }
        info!("[Lua] Script paused.");
    }

    pub fn resume_script(&mut self) {
        self.is_paused = false;
    }
}
