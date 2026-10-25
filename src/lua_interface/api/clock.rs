// clock.rs — `wall_time()`: real (wall-clock) seconds since the engine started
//
// Monotonic, with sub-millisecond resolution, so scripts can timestamp what they
// log (e.g. data collection against grSim or real robots, where ticks jitter).

use mlua::prelude::*;
use std::sync::OnceLock;
use std::time::Instant;

static START: OnceLock<Instant> = OnceLock::new();

pub(super) fn register_clock_functions(lua: &Lua) {
    let start = *START.get_or_init(Instant::now);
    let f = lua
        .create_function(move |_, ()| Ok(start.elapsed().as_secs_f64()))
        .unwrap();
    lua.globals().set("wall_time", f).unwrap();
}
