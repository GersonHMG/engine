// simulator — In-process simulator link (replaces grSim)
//
// Owns an `ssl_sim::Simulator` that advances in lock-step with the engine
// loop: commands in, one step, state written straight into the World. No
// UDP and no vision, so the engine can run it faster than real time.
//
// Robots are added with `sim add` in the console, or by teleporting one
// that does not exist yet (like grSim turning a robot on).

pub mod console;

use crate::types::{RobotCommand, Vec2D};
use crate::world::World;
use ssl_sim::{Ball, RobotId, SimConfig, SimState, Simulator, Team, Vec2};
use std::collections::{BTreeMap, HashMap};
use tracing::debug;

/// Kick speed for `kick_x`, the same the grSim link sends.
const KICK_SPEED: f64 = 3.0;

/// A robot placement: position, heading (rad).
type Pose = (Vec2, f64);

pub struct SimLink {
    sim: Simulator,
    /// Where each robot was last placed; `reset` puts it back there.
    robot_homes: BTreeMap<RobotId, Pose>,
    /// Where the ball was last placed.
    ball_home: Vec2,
}

impl SimLink {
    /// Starts with blue robot 0 behind the ball at the center.
    pub fn new(field_length_m: f64, field_width_m: f64) -> Self {
        let mut config = SimConfig::default();
        config.field.length = field_length_m;
        config.field.width = field_width_m;

        let mut link = Self {
            sim: Simulator::new(config, SimState::new(Vec::new(), Ball::new(Vec2::ZERO, Vec2::ZERO))),
            robot_homes: BTreeMap::new(),
            ball_home: Vec2::ZERO,
        };
        link.place_robot(RobotId::new(Team::Blue, 0), Vec2::new(-1.0, 0.0), 0.0);
        link
    }

    /// Hands this frame's commands to the simulated robots; robots without
    /// a command stop. Commands are in the robot frame, as for grSim.
    pub fn apply_commands(&mut self, commands: &HashMap<(i32, i32), RobotCommand>) {
        let ids: Vec<RobotId> = self.sim.state().robots().iter().map(|r| r.id).collect();
        for id in ids {
            let command = commands.get(&engine_id(id)).map(to_sim_command).unwrap_or_default();
            // The id comes from the simulator itself, so it exists.
            let _ = self.sim.set_command(id, command);
        }
    }

    /// Moves a robot, adding it if it is not simulated yet.
    pub fn teleport_robot(&mut self, id: i32, team: i32, x: f64, y: f64, orientation: f64) {
        match sim_id(id, team) {
            Some(id) => self.place_robot(id, Vec2::new(x, y), orientation),
            None => debug!("Simulator: invalid robot ({id}, {team}), teleport ignored"),
        }
    }

    pub fn teleport_ball(&mut self, x: f64, y: f64) {
        self.ball_home = Vec2::new(x, y);
        self.sim.teleport_ball(self.ball_home, Vec2::ZERO);
    }

    /// Advances the simulation by `dt` seconds and writes the result into
    /// the world, as the vision receiver would.
    pub fn step(&mut self, dt: f64, world: &mut World) {
        self.sim.step(dt);
        for event in self.sim.take_events() {
            debug!("Simulator: {event:?}");
        }

        let state = self.sim.state();
        for robot in state.robots() {
            let (id, team) = engine_id(robot.id);
            world.update_robot(
                id,
                team,
                to_vec2d(robot.position),
                robot.theta as f32,
                to_vec2d(robot.velocity),
                robot.omega as f32,
            );
        }
        world.update_ball(to_vec2d(state.ball.velocity), to_vec2d(state.ball.position));
    }

    /// Places a robot at rest (adding it if needed) and remembers the spot.
    fn place_robot(&mut self, id: RobotId, position: Vec2, theta: f64) {
        if self.sim.teleport_robot(id, position, theta).is_err() {
            // Not simulated yet: add it.
            let _ = self.sim.add_robot(id, position, theta);
        }
        self.robot_homes.insert(id, (position, theta));
    }

    /// Puts every robot and the ball back where they were last placed.
    fn reset(&mut self) {
        for (&id, &(position, theta)) in &self.robot_homes {
            let _ = self.sim.teleport_robot(id, position, theta);
        }
        self.sim.teleport_ball(self.ball_home, Vec2::ZERO);
    }

    fn remove_robot(&mut self, id: RobotId) -> Result<(), ssl_sim::SimError> {
        self.sim.remove_robot(id)?;
        self.robot_homes.remove(&id);
        Ok(())
    }
}

/// Engine (id, team) to simulator id; team 0 = blue, 1 = yellow.
fn sim_id(id: i32, team: i32) -> Option<RobotId> {
    let team = match team {
        0 => Team::Blue,
        1 => Team::Yellow,
        _ => return None,
    };
    u32::try_from(id).ok().map(|id| RobotId::new(team, id))
}

/// Simulator id to engine (id, team).
fn engine_id(id: RobotId) -> (i32, i32) {
    let team = match id.team {
        Team::Blue => 0,
        Team::Yellow => 1,
    };
    (id.id as i32, team)
}

fn to_sim_command(command: &RobotCommand) -> ssl_sim::RobotCommand {
    ssl_sim::RobotCommand {
        vt: command.motion.vx.unwrap_or(0.0),
        vn: command.motion.vy.unwrap_or(0.0),
        omega: command.motion.angular.unwrap_or(0.0),
        kick_speed: if command.kicker.kick_x { KICK_SPEED } else { 0.0 },
    }
}

fn to_vec2d(v: Vec2) -> Vec2D {
    Vec2D::new(v.x, v.y)
}
