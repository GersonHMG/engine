// sim.rs — `sim.*` functions for scripts run on the built-in simulator
// (headless, or the GUI with SIM on; without the simulator they return 0,
// nothing, nil or an error line)
//
// sim.time()       simulated time, s
// sim.events()     simulator events since the last call
// sim.finish(ok)   end the run after this tick; ok = false makes it fail (GUI: pauses the script)
// sim.truth(id, team)  the robot's true state, without vision noise or delay
//                  ({x, y, orientation, vel_x, vel_y, omega}, nil if not simulated)
// sim.console(line)    runs a console line (e.g. "sim vision delay 0.03"), returns its output lines

use crate::lua_interface::RunControl;
use crate::sender::radio::Radio;
use crate::sender::simulator::{console as simulator_console, engine_id};
use crate::world::World;
use mlua::prelude::*;
use ssl_sim::{Event, EventKind, RobotId};
use std::sync::{Arc, Mutex, RwLock};

pub(super) fn register_sim_functions(
    lua: &Lua,
    radio: Arc<Mutex<Radio>>,
    world: Arc<RwLock<World>>,
    control: Arc<RunControl>,
) {
    let sim_table = lua.create_table().unwrap();

    let r_truth = Arc::clone(&radio);
    let truth = lua
        .create_function(move |lua, (id, team): (i32, i32)| {
            let mut radio = r_truth.lock().map_err(|e| LuaError::external(format!("{e}")))?;
            let Some((position, theta, velocity, omega)) =
                radio.simulator_mut().and_then(|sim| sim.robot_truth(id, team))
            else {
                return Ok(None);
            };
            let t = lua.create_table()?;
            t.set("x", position.x)?;
            t.set("y", position.y)?;
            t.set("orientation", theta)?;
            t.set("vel_x", velocity.x)?;
            t.set("vel_y", velocity.y)?;
            t.set("omega", omega)?;
            Ok(Some(t))
        })
        .unwrap();
    sim_table.set("truth", truth).unwrap();

    let r_console = Arc::clone(&radio);
    let console = lua
        .create_function(move |_, line: String| {
            let mut radio = r_console.lock().map_err(|e| LuaError::external(format!("{e}")))?;
            let mut world = world.write().map_err(|e| LuaError::external(format!("{e}")))?;
            Ok(simulator_console::execute(&line, radio.simulator_mut(), &mut world))
        })
        .unwrap();
    sim_table.set("console", console).unwrap();

    let r1 = Arc::clone(&radio);
    let time = lua
        .create_function(move |_, ()| {
            let mut radio = r1.lock().map_err(|e| LuaError::external(format!("{e}")))?;
            Ok(radio.simulator_mut().map_or(0.0, |sim| sim.time()))
        })
        .unwrap();
    sim_table.set("time", time).unwrap();

    let events = lua
        .create_function(move |lua, ()| {
            let events = {
                let mut radio = radio.lock().map_err(|e| LuaError::external(format!("{e}")))?;
                radio.simulator_mut().map(|sim| sim.take_events()).unwrap_or_default()
            };
            let list = lua.create_table()?;
            for event in &events {
                list.push(event_table(lua, event)?)?;
            }
            Ok(list)
        })
        .unwrap();
    sim_table.set("events", events).unwrap();

    let finish = lua
        .create_function(move |_, ok: Option<bool>| {
            control.finish(ok.unwrap_or(true));
            Ok(())
        })
        .unwrap();
    sim_table.set("finish", finish).unwrap();

    lua.globals().set("sim", sim_table).unwrap();
}

/// `{time, kind, ...}` with robots as `id` and `team` (0 = blue, 1 = yellow).
fn event_table(lua: &Lua, event: &Event) -> LuaResult<LuaTable> {
    let table = lua.create_table()?;
    table.set("time", event.time)?;
    let set_robot = |robot: RobotId| -> LuaResult<()> {
        let (id, team) = engine_id(robot);
        table.set("id", id)?;
        table.set("team", team)
    };
    match &event.kind {
        EventKind::BallHitWall { position, speed } => {
            table.set("kind", "ball_hit_wall")?;
            table.set("x", position.x)?;
            table.set("y", position.y)?;
            table.set("speed", *speed)?;
        }
        EventKind::RobotHitWall { robot, position } => {
            table.set("kind", "robot_hit_wall")?;
            set_robot(*robot)?;
            table.set("x", position.x)?;
            table.set("y", position.y)?;
        }
        EventKind::RobotTouchedBall { robot, on_front } => {
            table.set("kind", "robot_touched_ball")?;
            set_robot(*robot)?;
            table.set("on_front", *on_front)?;
        }
        EventKind::Kick { robot, speed } => {
            table.set("kind", "kick")?;
            set_robot(*robot)?;
            table.set("speed", *speed)?;
        }
    }
    Ok(table)
}
