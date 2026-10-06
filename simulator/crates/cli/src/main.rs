//! `engine-sim`: runs simulator scenarios from the command line.
//!
//! The result goes to stdout as JSON, errors go to stderr and the exit
//! code is non-zero on failure, so agents and scripts can drive it.

mod replay;
mod report;

use anyhow::Result;
use clap::{Parser, Subcommand};
use replay::ReplayWriter;
use report::{FinalState, Report};
use ssl_sim::Simulator;
use ssl_sim_scenario::{Scenario, ScriptCursor, SCENARIO_ROBOT};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "engine-sim", version, about = "RoboCup SSL simulator")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run a scenario file and print the result as JSON.
    Run {
        /// Scenario file (TOML).
        scenario: PathBuf,
        /// Also write the trajectory as CSV (engine replay format).
        #[arg(long, value_name = "FILE")]
        replay: Option<PathBuf>,
        /// Indent the JSON output.
        #[arg(long)]
        pretty: bool,
    },
}

fn main() -> ExitCode {
    match execute(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn execute(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Run { scenario, replay, pretty } => {
            let scenario = Scenario::load(&scenario)?;
            let report = run(&scenario, replay)?;
            let json = if pretty {
                serde_json::to_string_pretty(&report)?
            } else {
                serde_json::to_string(&report)?
            };
            println!("{json}");
            Ok(())
        }
    }
}

/// Steps the scenario tick by tick, applying each command when its time comes.
fn run(scenario: &Scenario, replay: Option<PathBuf>) -> Result<Report> {
    let mut sim = Simulator::new(scenario.config.clone(), scenario.initial_state());
    let mut replay = replay.map(|path| ReplayWriter::create(&path)).transpose()?;
    let mut events = Vec::new();
    let mut script = ScriptCursor::default();

    let ticks = (scenario.duration / scenario.tick).round() as usize;
    for tick in 0..ticks {
        if let Some(command) = script.advance(scenario, tick as f64 * scenario.tick) {
            sim.set_command(SCENARIO_ROBOT, command)?;
        }

        sim.step(scenario.tick);
        events.extend(sim.take_events());
        if let Some(replay) = replay.as_mut() {
            replay.write_frame(sim.state())?;
        }
    }

    if let Some(replay) = replay {
        replay.finish()?;
    }

    Ok(Report {
        scenario: scenario.name.clone(),
        duration: sim.time(),
        ticks,
        final_state: FinalState::from_state(sim.state()),
        events,
    })
}
