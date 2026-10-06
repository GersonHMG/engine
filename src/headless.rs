// headless.rs — Runs a Lua script against the simulator, without the GUI
//
// `engine --headless script.lua`: the same tick as the GUI engine (Lua
// `process()`, commands, one simulator step of 1/60 s), as fast as the
// machine allows. The script's `print` goes to stdout, logs to stderr, and
// the run ends when the script calls `sim.finish(ok)` or at the time limit.

use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use tracing::error;

use crate::config::{load_field_config_from_exe, load_simulator_config};
use crate::game_controller::GameState;
use crate::lua_interface::{LuaInterface, RunControl, ScriptExecState};
use crate::sender::radio::Radio;
use crate::sender::simulator::SimLink;
use crate::world::World;

/// Prefix the Lua interface puts on `print` output.
const PRINT_PREFIX: &str = "[Lua][print] ";

/// Exit codes, so an agent can tell the outcomes apart.
pub const EXIT_OK: i32 = 0;
pub const EXIT_FAILED: i32 = 1;
pub const EXIT_SCRIPT_ERROR: i32 = 2;
pub const EXIT_TIME_LIMIT: i32 = 3;

/// Runs `script` until it calls `sim.finish` or `max_time` simulated seconds
/// pass. Returns the process exit code.
pub fn run(script: &str, max_time: f64, tick: Duration) -> i32 {
    let field = load_field_config_from_exe();
    let world = Arc::new(RwLock::new(World::new(6, 6, field.length_m, field.width_m)));
    let radio = Arc::new(Mutex::new(Radio::new(false, "", 0)));
    {
        let mut link = SimLink::new(load_simulator_config(field));
        link.record_events();
        radio.lock().unwrap().attach_simulator(link);
    }

    let (log_tx, mut log_rx) = tokio::sync::mpsc::channel::<String>(4096);
    let control = Arc::new(RunControl::default());
    let mut lua = LuaInterface::new(
        Arc::clone(&radio),
        Arc::clone(&world),
        Arc::new(Mutex::new(GameState::new())),
        Some(log_tx),
    );
    lua.set_run_control(Arc::clone(&control));

    let started = Instant::now();
    let loaded = lua.run_script(script);
    print_script_output(&mut log_rx);
    if loaded == ScriptExecState::Failed {
        return EXIT_SCRIPT_ERROR;
    }
    lua.resume_script();

    let mut ticks: u64 = 0;
    let code = loop {
        let state = lua.call_process();
        lua.take_draw_commands();
        print_script_output(&mut log_rx);
        if state == ScriptExecState::Failed {
            break EXIT_SCRIPT_ERROR;
        }

        {
            let mut radio = radio.lock().unwrap_or_else(|e| e.into_inner());
            let mut world = world.write().unwrap_or_else(|e| e.into_inner());
            radio.prepare_frame();
            radio.send_commands(&mut world);
            radio.step_simulator(&mut world, tick.as_secs_f64());
        }
        ticks += 1;

        if let Some(ok) = control.outcome() {
            break if ok { EXIT_OK } else { EXIT_FAILED };
        }
        if ticks as f64 * tick.as_secs_f64() >= max_time {
            error!("Time limit of {max_time} s reached without sim.finish()");
            break EXIT_TIME_LIMIT;
        }
    };

    eprintln!(
        "headless: exit {code} after {:.3} s simulated ({ticks} ticks) in {:.2} s",
        ticks as f64 * tick.as_secs_f64(),
        started.elapsed().as_secs_f64(),
    );
    code
}

/// `print` output to stdout; the interface's own messages are already
/// logged to stderr by tracing.
fn print_script_output(log_rx: &mut tokio::sync::mpsc::Receiver<String>) {
    while let Ok(line) = log_rx.try_recv() {
        if let Some(text) = line.strip_prefix(PRINT_PREFIX) {
            println!("{text}");
        }
    }
}
