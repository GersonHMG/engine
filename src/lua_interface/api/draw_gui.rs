use crate::types::DrawCommand;
use mlua::prelude::*;
use std::sync::{Arc, Mutex};

pub(super) fn register_draw_gui_functions(
    lua: &Lua,
    draw_commands: Arc<Mutex<Vec<DrawCommand>>>,
) {
    let globals = lua.globals();

    // ── draw_point(x, y) ──
    {
        let dc = Arc::clone(&draw_commands);
        let f = lua
            .create_function(move |_, (x, y): (f64, f64)| {
                dc.lock()
                    .map_err(|e| LuaError::external(format!("{e}")))?
                    .push(DrawCommand::Point { x, y });
                Ok(())
            })
            .unwrap();
        globals.set("draw_point", f).unwrap();
    }

    // ── highlight_robot(id, team) ──
    {
        let dc = Arc::clone(&draw_commands);
        let f = lua
            .create_function(move |_, (id, team): (i32, i32)| {
                dc.lock()
                    .map_err(|e| LuaError::external(format!("{e}")))?
                    .push(DrawCommand::HighlightRobot { id, team });
                Ok(())
            })
            .unwrap();
        globals.set("highlight_robot", f).unwrap();
    }

    // ── draw_line({{x, y}, {x, y}, ...}) ──
    {
        let dc = Arc::clone(&draw_commands);
        let f = lua
            .create_function(move |_, tbl: LuaTable| {
                let mut points = Vec::new();
                for pair in tbl.sequence_values::<LuaTable>() {
                    let pt = pair.map_err(|e| LuaError::external(format!("{e}")))?;
                    let x: f64 = pt.get(1).or_else(|_| pt.get("x"))?;
                    let y: f64 = pt.get(2).or_else(|_| pt.get("y"))?;
                    points.push([x, y]);
                }
                if points.len() >= 2 {
                    dc.lock()
                        .map_err(|e| LuaError::external(format!("{e}")))?
                        .push(DrawCommand::Line { points });
                }
                Ok(())
            })
            .unwrap();
        globals.set("draw_line", f).unwrap();
    }
}