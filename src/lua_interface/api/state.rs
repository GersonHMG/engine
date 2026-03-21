use crate::game_controller::GameState;
use crate::world::World;
use mlua::prelude::*;
use std::sync::{Arc, Mutex, RwLock};

pub(super) fn register_robot_state_functions(
    lua: &Lua,
    world: Arc<RwLock<World>>,
    game_state: Arc<Mutex<GameState>>,
) {
    let globals = lua.globals();

    // ── get_robot_state(id, team) ──
    {
        let w = Arc::clone(&world);
        let f = lua
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
        let f = lua
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
        let f = lua
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
                    result.set(i + 1, t)?;
                }
                Ok(result)
            })
            .unwrap();
        globals.set("get_blue_team_state", f).unwrap();
    }

    // ── get_yellow_team_state() ──
    {
        let w = Arc::clone(&world);
        let f = lua
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
        let f = lua
            .create_function(move |_, ()| {
                let gs = gs.lock().map_err(|e| LuaError::external(format!("{e}")))?;
                Ok(gs.get_ref_message().to_string())
            })
            .unwrap();
        globals.set("get_ref_message", f).unwrap();
    }
}