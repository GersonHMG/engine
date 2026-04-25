use crate::sender::radio::Radio;
use mlua::prelude::*;
use std::sync::{Arc, Mutex};

pub(super) fn register_grsim_functions(lua: &Lua, radio: Arc<Mutex<Radio>>) {
    let globals = lua.globals();

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
