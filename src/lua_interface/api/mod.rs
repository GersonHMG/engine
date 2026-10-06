mod control;
mod draw_gui;
mod grsim;
mod sim;
mod state;

use crate::game_controller::GameState;
use crate::lua_interface::RunControl;
use crate::sender::radio::Radio;
use crate::types::DrawCommand;
use crate::world::World;
use mlua::prelude::*;
use std::sync::{Arc, Mutex, RwLock};

pub(super) fn register_api_functions(
    lua: &Lua,
    radio: Arc<Mutex<Radio>>,
    world: Arc<RwLock<World>>,
    game_state: Arc<Mutex<GameState>>,
    draw_commands: Arc<Mutex<Vec<DrawCommand>>>,
    run_control: Option<Arc<RunControl>>,
) {
    draw_gui::register_draw_gui_functions(lua, draw_commands);
    state::register_robot_state_functions(lua, Arc::clone(&world), game_state);
    control::register_control_functions(lua, Arc::clone(&radio), world);
    if let Some(control) = run_control {
        sim::register_sim_functions(lua, Arc::clone(&radio), control);
    }
    grsim::register_grsim_functions(lua, radio);
}