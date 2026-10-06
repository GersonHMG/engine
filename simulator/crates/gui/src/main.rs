//! `engine-sim-gui [scenario.toml]`: shows the simulator's field in a
//! window, playing the scenario in real time if one is given.

mod app;
mod field_view;

use app::App;
use ssl_sim_scenario::Scenario;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Mutex;

fn main() -> ExitCode {
    let scenario = match std::env::args().nth(1).map(PathBuf::from) {
        Some(path) => match Scenario::load(&path) {
            Ok(scenario) => Some(scenario),
            Err(error) => {
                eprintln!("error: {error:#}");
                return ExitCode::FAILURE;
            }
        },
        None => None,
    };

    // iced may call the boot function more than once; the scenario is used once.
    let scenario = Mutex::new(scenario);
    let result = iced::application(
        move || App::new(scenario.lock().unwrap().take()),
        App::update,
        App::view,
    )
    .title("SSL Simulator")
    .subscription(App::subscription)
    .window_size((640.0, 440.0))
    .run();

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
