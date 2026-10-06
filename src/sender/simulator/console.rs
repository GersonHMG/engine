// console.rs — `sim ...` commands typed in the engine console
//
// Each command returns the lines to print back in the console.

use super::{engine_id, SimLink};
use crate::world::World;
use serde::de::DeserializeOwned;
use serde::Serialize;
use ssl_sim::{RobotId, Team, Vec2};
use std::f64::consts::PI;

const HELP: &[&str] = &[
    "sim status                              time, robots and ball",
    "sim add <blue|yellow> <id> [x y [θ]]    add a robot (default: a free spot on its side)",
    "sim remove <blue|yellow> <id>           remove a robot",
    "sim reset                               robots and ball back to where they were last placed",
    "sim config                              all simulator parameters",
    "sim robot [<parameter> <value>]         show or change the robot profile",
    "sim ball [<parameter> <value>]          show or change the ball profile",
];

/// Runs one console line and returns what to print.
pub fn execute(line: &str, simulator: Option<&mut SimLink>, world: &mut World) -> Vec<String> {
    let words: Vec<&str> = line.split_whitespace().collect();
    let result = match words.as_slice() {
        [] => Ok(Vec::new()),
        ["sim"] | ["sim", "help"] | ["help"] => Ok(HELP.iter().map(|s| s.to_string()).collect()),
        ["sim", args @ ..] => match simulator {
            Some(simulator) => run(simulator, world, args),
            None => Err("the simulator is off: click SIM in the toolbar".to_string()),
        },
        [other, ..] => Err(format!("unknown command `{other}`, type `sim help`")),
    };
    result.unwrap_or_else(|error| vec![format!("error: {error}")])
}

fn run(simulator: &mut SimLink, world: &mut World, args: &[&str]) -> Result<Vec<String>, String> {
    match args {
        ["status"] => Ok(status(simulator)),
        ["add", team, id, rest @ ..] => {
            let id = RobotId::new(parse_team(team)?, parse_number(id, "id")?);
            if simulator.sim.state().robot(id).is_some() {
                return Err(format!("robot {id} already exists"));
            }
            let (position, theta) = match rest {
                [] => (free_spot(simulator, id.team), facing(id.team)),
                [x, y] => (Vec2::new(parse_number(x, "x")?, parse_number(y, "y")?), facing(id.team)),
                [x, y, theta] => (
                    Vec2::new(parse_number(x, "x")?, parse_number(y, "y")?),
                    parse_number(theta, "θ")?,
                ),
                _ => return Err("usage: sim add <blue|yellow> <id> [x y [θ]]".to_string()),
            };
            simulator.place_robot(id, position, theta);
            Ok(vec![format!("added {id} at ({:.2}, {:.2})", position.x, position.y)])
        }
        ["remove", team, id] => {
            let id = RobotId::new(parse_team(team)?, parse_number(id, "id")?);
            simulator.remove_robot(id).map_err(|e| e.to_string())?;
            let (engine_id, engine_team) = engine_id(id);
            world.remove_robot(engine_id, engine_team);
            Ok(vec![format!("removed {id}")])
        }
        ["reset"] => {
            simulator.reset();
            Ok(vec!["robots and ball reset".to_string()])
        }
        ["config"] => {
            let config = simulator.sim.config();
            let mut lines = vec!["field:".to_string()];
            lines.extend(profile_lines(&config.field));
            lines.push("robot:".to_string());
            lines.extend(profile_lines(&config.robot));
            lines.push("ball:".to_string());
            lines.extend(profile_lines(&config.ball));
            Ok(lines)
        }
        ["robot"] => Ok(profile_lines(&simulator.sim.config().robot)),
        ["robot", name, value] => {
            let mut config = simulator.sim.config().clone();
            config.robot = with_value(&config.robot, name, parse_number(value, name)?)?;
            simulator.sim.set_config(config);
            Ok(vec![format!("robot {name} = {value}")])
        }
        ["ball"] => Ok(profile_lines(&simulator.sim.config().ball)),
        ["ball", name, value] => {
            let mut config = simulator.sim.config().clone();
            config.ball = with_value(&config.ball, name, parse_number(value, name)?)?;
            simulator.sim.set_config(config);
            Ok(vec![format!("ball {name} = {value}")])
        }
        [other, ..] => Err(format!("unknown sim command `{other}`, type `sim help`")),
        [] => unreachable!("handled by `execute`"),
    }
}

fn status(simulator: &SimLink) -> Vec<String> {
    let state = simulator.sim.state();
    let mut lines = vec![format!("time {:.2} s, {} robots", state.time, state.robots().len())];
    for robot in state.robots() {
        lines.push(format!(
            "{:<10} ({:6.2}, {:6.2})  θ {:5.2}  {:.2} m/s",
            robot.id.to_string(),
            robot.position.x,
            robot.position.y,
            robot.theta,
            robot.velocity.length(),
        ));
    }
    let ball = &state.ball;
    lines.push(format!(
        "{:<10} ({:6.2}, {:6.2})  {:.2} m/s",
        "ball",
        ball.position.x,
        ball.position.y,
        ball.velocity.length(),
    ));
    lines
}

/// Each parameter of a profile as `name = value`.
fn profile_lines<T: Serialize>(profile: &T) -> Vec<String> {
    match serde_json::to_value(profile) {
        Ok(serde_json::Value::Object(fields)) => {
            fields.iter().map(|(name, value)| format!("  {name} = {value}")).collect()
        }
        _ => vec!["  (unavailable)".to_string()],
    }
}

/// A copy of `profile` with parameter `name` set to `value`.
fn with_value<T: Serialize + DeserializeOwned>(profile: &T, name: &str, value: f64) -> Result<T, String> {
    if !(value.is_finite() && value >= 0.0) {
        return Err(format!("{name} must be a number >= 0"));
    }
    let mut json = serde_json::to_value(profile).map_err(|e| e.to_string())?;
    let fields = json.as_object_mut().ok_or("profile is not a table")?;
    if !fields.contains_key(name) {
        let names: Vec<&str> = fields.keys().map(String::as_str).collect();
        return Err(format!("unknown parameter `{name}`, one of: {}", names.join(", ")));
    }
    fields.insert(name.to_string(), serde_json::json!(value));
    serde_json::from_value(json).map_err(|e| e.to_string())
}

fn parse_team(word: &str) -> Result<Team, String> {
    match word.to_ascii_lowercase().as_str() {
        "blue" | "b" => Ok(Team::Blue),
        "yellow" | "y" => Ok(Team::Yellow),
        _ => Err(format!("team must be blue or yellow, got `{word}`")),
    }
}

fn parse_number<T: std::str::FromStr>(word: &str, name: &str) -> Result<T, String> {
    word.parse().map_err(|_| format!("{name} must be a number, got `{word}`"))
}

/// Robots face the opponent's goal: blue towards +x, yellow towards -x.
fn facing(team: Team) -> f64 {
    match team {
        Team::Blue => 0.0,
        Team::Yellow => PI,
    }
}

/// A spot on the team's half, 1.5 m from the center line, not next to
/// another robot.
fn free_spot(simulator: &SimLink, team: Team) -> Vec2 {
    const SPACING: f64 = 0.5;
    let x = match team {
        Team::Blue => -1.5,
        Team::Yellow => 1.5,
    };
    let robots = simulator.sim.state().robots();
    // y = 0, +0.5, -0.5, +1.0, -1.0, ... up to ±2.5
    let offsets = std::iter::once(0.0).chain((1..=5).flat_map(|k| {
        let y = SPACING * f64::from(k);
        [y, -y]
    }));
    offsets
        .map(|y| Vec2::new(x, y))
        .find(|spot| robots.iter().all(|r| (r.position - *spot).length() > SPACING * 0.9))
        .unwrap_or(Vec2::new(x, 0.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (SimLink, World) {
        (SimLink::new(9.0, 6.0), World::new(0, 0, 9.0, 6.0))
    }

    fn run_line(simulator: &mut SimLink, world: &mut World, line: &str) -> Vec<String> {
        execute(line, Some(simulator), world)
    }

    #[test]
    fn needs_the_simulator_on() {
        let mut world = World::new(0, 0, 9.0, 6.0);
        assert_eq!(execute("sim status", None, &mut world), ["error: the simulator is off: click SIM in the toolbar"]);
        assert_eq!(execute("sim help", None, &mut world).len(), HELP.len());
    }

    #[test]
    fn adds_and_removes_robots() {
        let (mut sim, mut world) = setup();

        assert_eq!(run_line(&mut sim, &mut world, "sim add yellow 2 1.0 -0.5"), ["added yellow 2 at (1.00, -0.50)"]);
        assert_eq!(run_line(&mut sim, &mut world, "sim add y 2"), ["error: robot yellow 2 already exists"]);
        assert_eq!(run_line(&mut sim, &mut world, "sim add b 1"), ["added blue 1 at (-1.50, 0.00)"]);

        let status = run_line(&mut sim, &mut world, "sim status");
        assert!(status[0].contains("3 robots"), "{status:?}");

        assert_eq!(run_line(&mut sim, &mut world, "sim remove blue 1"), ["removed blue 1"]);
        assert_eq!(run_line(&mut sim, &mut world, "sim remove blue 1"), ["error: there is no robot blue 1"]);
    }

    #[test]
    fn changes_profiles() {
        let (mut sim, mut world) = setup();

        assert_eq!(run_line(&mut sim, &mut world, "sim robot max_speed 2.0"), ["robot max_speed = 2.0"]);
        assert_eq!(sim.sim.config().robot.max_speed, 2.0);
        assert_eq!(run_line(&mut sim, &mut world, "sim ball roll_decel 0.5"), ["ball roll_decel = 0.5"]);
        assert_eq!(sim.sim.config().ball.roll_decel, 0.5);

        let unknown = run_line(&mut sim, &mut world, "sim robot speed 2");
        assert!(unknown[0].starts_with("error: unknown parameter `speed`, one of:"), "{unknown:?}");
        assert_eq!(run_line(&mut sim, &mut world, "sim ball radius -1"), ["error: radius must be a number >= 0"]);
    }

    #[test]
    fn reset_returns_robots_and_ball_to_where_they_were_placed() {
        let (mut sim, mut world) = setup();
        sim.teleport_ball(1.0, 1.0);
        sim.sim.teleport_ball(Vec2::new(-2.0, 0.5), Vec2::new(1.0, 0.0));
        let blue0 = RobotId::new(Team::Blue, 0);
        sim.sim.teleport_robot(blue0, Vec2::new(3.0, 2.0), 1.0).unwrap();

        assert_eq!(run_line(&mut sim, &mut world, "sim reset"), ["robots and ball reset"]);

        let state = sim.sim.state();
        assert_eq!(state.ball.position, Vec2::new(1.0, 1.0));
        assert_eq!(state.ball.velocity, Vec2::ZERO);
        assert_eq!(state.robot(blue0).unwrap().position, Vec2::new(-1.0, 0.0));
    }

    #[test]
    fn rejects_unknown_commands() {
        let (mut sim, mut world) = setup();
        assert_eq!(run_line(&mut sim, &mut world, "fly"), ["error: unknown command `fly`, type `sim help`"]);
        assert_eq!(run_line(&mut sim, &mut world, "sim fly"), ["error: unknown sim command `fly`, type `sim help`"]);
        assert!(run_line(&mut sim, &mut world, "   ").is_empty());
    }
}
