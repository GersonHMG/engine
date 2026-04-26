use crate::motion::Motion;
use crate::sender::radio::Radio;
use crate::types::{KickerCommand, MotionCommand, Vec2D};
use crate::world::World;
use mlua::prelude::*;
use std::sync::{Arc, Mutex, RwLock};

pub(super) fn register_control_functions(
    lua: &Lua,
    radio: Arc<Mutex<Radio>>,
    world: Arc<RwLock<World>>,
) {
    let globals = lua.globals();

    // ── send_velocity(id, team, vx, vy, omega) ──
    {
        let r = Arc::clone(&radio);
        let w = Arc::clone(&world);
        let f = lua
            .create_function(move |_, (id, team, vx, vy, omega): (i32, i32, f64, f64, f64)| {
                let w = w.read().map_err(|e| LuaError::external(format!("{e}")))?;
                let rs = w.get_robot_state(id, team);
                if !rs.active {
                    return Ok(());
                }
                let mut cmd = MotionCommand::new(id, team, vx, vy);
                cmd.angular = Some(omega);
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
        let f = lua
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
        let f = lua
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

    // ── face_to(id, team, {x=, y=}, kp?, ki?, kd?) ──
    {
        let r = Arc::clone(&radio);
        let w = Arc::clone(&world);
        let f = lua
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
        let f = lua
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
        let f = lua
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
        let f = lua
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
}