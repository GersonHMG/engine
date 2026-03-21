use crate::motion::Motion;
use crate::radio::Radio;
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

    // ── motion(id, team, {x=, y=}, kp_x?, ki_x?, kp_y?, ki_y?) ──
    {
        let r = Arc::clone(&radio);
        let w = Arc::clone(&world);
        let f = lua
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

    // ── bangbang_trajectory(id, team, v_max, a_max, {{x, y}, ...}) ──
    {
        let r = Arc::clone(&radio);
        let w = Arc::clone(&world);
        let f = lua
            .create_function(
                move |_, (id, team, v_max, a_max, path_tbl): (i32, i32, f64, f64, LuaTable)| {
                    if v_max <= 0.0 || a_max <= 0.0 {
                        return Ok(());
                    }

                    let mut path = Vec::new();
                    for pair in path_tbl.sequence_values::<LuaTable>() {
                        let pt = pair.map_err(|e| LuaError::external(format!("{e}")))?;
                        let x: f64 = pt.get(1).or_else(|_| pt.get("x"))?;
                        let y: f64 = pt.get(2).or_else(|_| pt.get("y"))?;
                        path.push(Vec2D::new(x, y));
                    }

                    if path.is_empty() {
                        return Ok(());
                    }

                    let w = w.read().map_err(|e| LuaError::external(format!("{e}")))?;
                    let rs = w.get_robot_state(id, team);
                    if !rs.active {
                        return Ok(());
                    }

                    let motion = Motion::new();
                    let cmd = motion.bangbang_trajectory(&rs, id, team, v_max, a_max, path);
                    r.lock()
                        .map_err(|e| LuaError::external(format!("{e}")))?
                        .add_motion_command(cmd);
                    Ok(())
                },
            )
            .unwrap();
        globals.set("bangbang_trajectory", f).unwrap();
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

    // ── grsim.teleport_robot(id, team, x, y, dir) ──
    // ── grsim.teleport_ball(x, y) ──
    {
        let grsim_table = lua.create_table().unwrap();

        let r1 = Arc::clone(&radio);
        let teleport_robot = lua
            .create_function(move |_, (id, team, x, y, dir): (i32, i32, f64, f64, f64)| {
                r1.lock()
                    .map_err(|e| LuaError::external(format!("{e}")))?
                    .teleport_robot(id, team, x, y, dir);
                Ok(())
            })
            .unwrap();
        grsim_table.set("teleport_robot", teleport_robot).unwrap();

        let r2 = Arc::clone(&radio);
        let teleport_ball = lua
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