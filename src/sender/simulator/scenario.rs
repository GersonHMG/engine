// scenario.rs — Saved field setups (`sim scenario save/load/list`)
//
// A scenario is where every robot and the ball are, kept as a TOML file in
// `scenarios/` under the working directory (the repository root with
// `cargo run`).

use super::SimLink;
use crate::world::World;
use serde::{Deserialize, Serialize};
use ssl_sim::{RobotId, Team, Vec2};
use std::fs;
use std::path::PathBuf;

const SCENARIO_DIR: &str = "scenarios";

const HEADER: &str = "\
# Field setup for the engine's simulator: where every robot and the ball are.
# Load it with `sim scenario load <name>` in the Lua console.
";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    pub ball: BallPlacement,
    #[serde(default)]
    pub robots: Vec<RobotPlacement>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BallPlacement {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RobotPlacement {
    pub team: Team,
    pub id: u32,
    pub x: f64,
    pub y: f64,
    /// Heading, rad.
    #[serde(default)]
    pub theta: f64,
}

impl SimLink {
    /// Where every robot and the ball are now.
    pub(super) fn scenario(&self) -> Scenario {
        let state = self.sim.state();
        Scenario {
            ball: BallPlacement { x: state.ball.position.x, y: state.ball.position.y },
            robots: state
                .robots()
                .iter()
                .map(|robot| RobotPlacement {
                    team: robot.id.team,
                    id: robot.id.id,
                    x: robot.position.x,
                    y: robot.position.y,
                    theta: robot.theta,
                })
                .collect(),
        }
    }

    /// Places the robots and the ball of `scenario`, at rest; robots that
    /// are not in it are removed. `reset` then returns to this setup.
    pub(super) fn load_scenario(&mut self, scenario: &Scenario, world: &mut World) {
        let wanted: Vec<RobotId> = scenario.robots.iter().map(|r| RobotId::new(r.team, r.id)).collect();
        let unwanted: Vec<RobotId> = self
            .sim
            .state()
            .robots()
            .iter()
            .map(|r| r.id)
            .filter(|id| !wanted.contains(id))
            .collect();
        for id in unwanted {
            let _ = self.remove_robot(id);
            let (engine_id, engine_team) = super::engine_id(id);
            world.remove_robot(engine_id, engine_team);
        }

        for robot in &scenario.robots {
            self.place_robot(RobotId::new(robot.team, robot.id), Vec2::new(robot.x, robot.y), robot.theta);
        }
        self.teleport_ball(scenario.ball.x, scenario.ball.y);
    }
}

/// Writes `scenarios/<name>.toml`, returning the file written.
pub fn save(name: &str, scenario: &Scenario) -> Result<PathBuf, String> {
    let path = path_for(name)?;
    fs::create_dir_all(SCENARIO_DIR).map_err(|e| format!("cannot create {SCENARIO_DIR}/: {e}"))?;
    fs::write(&path, to_text(scenario)?).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    Ok(path)
}

pub fn load(name: &str) -> Result<Scenario, String> {
    let path = path_for(name)?;
    let text = fs::read_to_string(&path).map_err(|_| format!("no scenario `{name}` ({} not found)", path.display()))?;
    from_text(&text).map_err(|e| format!("invalid {}: {e}", path.display()))
}

/// Names of the saved scenarios, sorted.
pub fn list() -> Vec<String> {
    let Ok(entries) = fs::read_dir(SCENARIO_DIR) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "toml"))
        .filter_map(|path| path.file_stem().map(|stem| stem.to_string_lossy().into_owned()))
        .collect();
    names.sort();
    names
}

fn to_text(scenario: &Scenario) -> Result<String, String> {
    let body = toml::to_string_pretty(scenario).map_err(|e| e.to_string())?;
    Ok(format!("{HEADER}\n{body}"))
}

fn from_text(text: &str) -> Result<Scenario, String> {
    toml::from_str(text).map_err(|e| e.to_string())
}

/// Names are letters, digits, `-` and `_`, so they are safe file names.
fn path_for(name: &str) -> Result<PathBuf, String> {
    let valid = !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !valid {
        return Err(format!("scenario names use letters, digits, - and _, got `{name}`"));
    }
    Ok(PathBuf::from(SCENARIO_DIR).join(format!("{name}.toml")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ssl_sim::SimConfig;

    #[test]
    fn a_saved_setup_loads_back() {
        let mut original = SimLink::new(SimConfig::default());
        original.place_robot(RobotId::new(Team::Yellow, 3), Vec2::new(1.5, -0.5), 3.0);
        original.teleport_ball(0.25, 0.75);
        let scenario = original.scenario();

        let text = to_text(&scenario).unwrap();
        assert_eq!(from_text(&text), Ok(scenario.clone()));

        // A link with other robots ends up with exactly the scenario's.
        let mut world = World::new(0, 0, 9.0, 6.0);
        let mut other = SimLink::new(SimConfig::default());
        other.place_robot(RobotId::new(Team::Blue, 5), Vec2::new(-2.0, 1.0), 0.0);
        other.load_scenario(&scenario, &mut world);
        assert_eq!(other.scenario(), scenario);
    }

    #[test]
    fn names_must_be_safe_file_names() {
        assert!(path_for("kickoff_2v2").is_ok());
        assert!(path_for("").is_err());
        assert!(path_for("../secrets").is_err());
        assert!(path_for("a b").is_err());
    }
}
