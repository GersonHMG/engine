use crate::types::DrawCommand;
use mlua::prelude::*;
use std::sync::{Arc, Mutex};

fn parse_line_color(tbl: &LuaTable) -> LuaResult<[f32; 3]> {
    let r: f64 = tbl.get(1).or_else(|_| tbl.get("r"))?;
    let g: f64 = tbl.get(2).or_else(|_| tbl.get("g"))?;
    let b: f64 = tbl.get(3).or_else(|_| tbl.get("b"))?;
    Ok([
        r.clamp(0.0, 1.0) as f32,
        g.clamp(0.0, 1.0) as f32,
        b.clamp(0.0, 1.0) as f32,
    ])
}

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

    // ── draw_line(points[, draw_points_between][, color]) ──
    {
        let dc = Arc::clone(&draw_commands);
        let f = lua
            .create_function(move |_, (tbl, arg2, arg3): (LuaTable, Option<LuaValue>, Option<LuaValue>)| {
                let mut points = Vec::new();
                for pair in tbl.sequence_values::<LuaTable>() {
                    let pt = pair.map_err(|e| LuaError::external(format!("{e}")))?;
                    let x: f64 = pt.get(1).or_else(|_| pt.get("x"))?;
                    let y: f64 = pt.get(2).or_else(|_| pt.get("y"))?;
                    points.push([x, y]);
                }

                let mut draw_points_between = false;
                let mut color = None;

                if let Some(value) = arg2 {
                    match value {
                        LuaValue::Boolean(v) => draw_points_between = v,
                        LuaValue::Table(tbl) => color = Some(parse_line_color(&tbl)?),
                        LuaValue::Nil => {}
                        _ => {
                            return Err(LuaError::external(
                                "draw_line second argument must be a boolean or color table",
                            ))
                        }
                    }
                }

                if let Some(value) = arg3 {
                    match value {
                        LuaValue::Table(tbl) => color = Some(parse_line_color(&tbl)?),
                        LuaValue::Nil => {}
                        _ => {
                            return Err(LuaError::external(
                                "draw_line third argument must be a color table",
                            ))
                        }
                    }
                }

                if points.len() >= 2 {
                    dc.lock()
                        .map_err(|e| LuaError::external(format!("{e}")))?
                        .push(DrawCommand::Line {
                            points,
                            draw_points_between,
                            color,
                        });
                }
                Ok(())
            })
            .unwrap();
        globals.set("draw_line", f).unwrap();
    }
}