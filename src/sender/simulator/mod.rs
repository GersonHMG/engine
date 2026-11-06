// simulator — In-process simulator link (replaces grSim)
//
// Owns an `ssl_sim::Simulator` that advances in lock-step with the engine
// loop: commands in, one step, state written straight into the World. No
// UDP and no vision, so the engine can run it faster than real time.
//
// Robots are added with `sim add` in the console, or by teleporting one
// that does not exist yet (like grSim turning a robot on).
//
// The `[vision]` and `[radio]` settings make it less perfect, like real
// robots: commands reach the robots `radio.delay` late, and the AI sees each
// frame `vision.delay` late, with gaussian noise on positions and headings.
// Velocities stay exact (real vision gets them from the Kalman filter).

pub mod console;
pub mod scenario;

use crate::types::{RobotCommand, Vec2D};
use crate::world::World;
use ssl_sim::{Ball, Event, RobotId, SimConfig, SimState, Simulator, Team, Vec2};
use std::collections::{BTreeMap, HashMap, VecDeque};
use tracing::debug;

/// A robot placement: position, heading (rad).
type Pose = (Vec2, f64);

pub struct SimLink {
    sim: Simulator,
    /// Where each robot was last placed; `reset` puts it back there.
    robot_homes: BTreeMap<RobotId, Pose>,
    /// Where the ball was last placed.
    ball_home: Vec2,
    /// Events kept for `take_events`; `None` = not recorded (they would pile
    /// up with nobody reading them).
    events: Option<Vec<Event>>,
    /// Commands sent but not received yet: (time they arrive, every robot's command).
    in_flight: VecDeque<(f64, Vec<(RobotId, ssl_sim::RobotCommand)>)>,
    /// Vision frames taken but not received yet, oldest first.
    frames: VecDeque<Frame>,
    /// Vision noise; fixed seed, so runs repeat exactly.
    rng: Gaussian,
}

/// What the camera saw at one moment, noise included.
struct Frame {
    time: f64,
    /// (engine id, team), position, heading, velocity, angular velocity.
    robots: Vec<((i32, i32), Vec2D, f32, Vec2D, f32)>,
    ball_position: Vec2D,
    ball_velocity: Vec2D,
}

/// Slack when comparing simulated times that are sums of ticks, s.
const TIME_EPS: f64 = 1e-9;

impl SimLink {
    /// Starts with blue robot 0 behind the ball at the center.
    pub fn new(config: SimConfig) -> Self {
        let mut link = Self {
            sim: Simulator::new(config, SimState::new(Vec::new(), Ball::new(Vec2::ZERO, Vec2::ZERO))),
            robot_homes: BTreeMap::new(),
            ball_home: Vec2::ZERO,
            events: None,
            in_flight: VecDeque::new(),
            frames: VecDeque::new(),
            rng: Gaussian::new(0x5157_4d1c_0ffe_e5ed),
        };
        link.place_robot(RobotId::new(Team::Blue, 0), Vec2::new(-1.0, 0.0), 0.0);
        link
    }

    /// The physical parameters the simulator runs with.
    pub fn config(&self) -> &SimConfig {
        self.sim.config()
    }

    /// Hands this frame's commands to the simulated robots; robots without
    /// a command stop. Commands are in the robot frame, as for grSim.
    pub fn apply_commands(&mut self, commands: &HashMap<(i32, i32), RobotCommand>) {
        let ids: Vec<RobotId> = self.sim.state().robots().iter().map(|r| r.id).collect();
        // `kickx` kicks at the configured flat-kick speed (simulator.toml, robot.kick_x_speed).
        let kick_speed = self.sim.config().robot.kick_x_speed;
        let frame: Vec<(RobotId, ssl_sim::RobotCommand)> = ids
            .into_iter()
            .map(|id| (id, commands.get(&engine_id(id)).map(|c| to_sim_command(c, kick_speed)).unwrap_or_default()))
            .collect();

        // With a radio delay the commands arrive later; until newer ones
        // arrive, robots keep doing what they were last told.
        let now = self.sim.time();
        self.in_flight.push_back((now + self.sim.config().radio.delay, frame));
        let mut arrived = None;
        while self.in_flight.front().is_some_and(|(at, _)| *at <= now + TIME_EPS) {
            arrived = self.in_flight.pop_front().map(|(_, frame)| frame);
        }
        for (id, command) in arrived.into_iter().flatten() {
            // Robots removed since the command was sent are skipped.
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
        let events = self.sim.take_events();
        match self.events.as_mut() {
            Some(recorded) => recorded.extend(events),
            None => events.iter().for_each(|event| debug!("Simulator: {event:?}")),
        }

        let frame = self.capture();
        self.frames.push_back(frame);
        // The newest frame that is at least `vision.delay` old is the one
        // the AI has; older ones are dropped.
        let seen_until = self.sim.time() - self.sim.config().vision.delay + TIME_EPS;
        while self.frames.get(1).is_some_and(|next| next.time <= seen_until) {
            self.frames.pop_front();
        }
        if let Some(frame) = self.frames.front().filter(|f| f.time <= seen_until) {
            write_frame(frame, world);
        }
    }

    /// The simulation as the camera sees it now, with the vision noise.
    fn capture(&mut self) -> Frame {
        let vision = self.sim.config().vision.clone();
        let state = self.sim.state();
        let rng = &mut self.rng;
        let robots = state
            .robots()
            .iter()
            .map(|robot| {
                let position = Vec2D::new(
                    robot.position.x + rng.sample(vision.position_noise),
                    robot.position.y + rng.sample(vision.position_noise),
                );
                let theta = wrap_angle(robot.theta + rng.sample(vision.angle_noise)) as f32;
                (engine_id(robot.id), position, theta, to_vec2d(robot.velocity), robot.omega as f32)
            })
            .collect();
        let ball_position = Vec2D::new(
            state.ball.position.x + rng.sample(vision.position_noise),
            state.ball.position.y + rng.sample(vision.position_noise),
        );
        Frame {
            time: state.time,
            robots,
            ball_position,
            ball_velocity: to_vec2d(state.ball.velocity),
        }
    }

    /// Forgets commands and frames still on their way, so a teleport or a
    /// scenario load is not undone by what was sent or seen before it.
    pub(super) fn clear_in_flight(&mut self) {
        self.in_flight.clear();
        self.frames.clear();
    }

    /// Writes the simulation into the world without advancing it, so robots
    /// and the ball placed while it is paused show up. No noise, no delay.
    pub fn write_world(&self, world: &mut World) {
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

    /// A robot's true state (no vision noise, no delay): position, heading,
    /// velocity, angular velocity. None if it is not simulated.
    pub fn robot_truth(&self, id: i32, team: i32) -> Option<(Vec2D, f64, Vec2D, f64)> {
        let id = sim_id(id, team)?;
        let robot = self.sim.state().robots().iter().find(|r| r.id == id)?;
        Some((to_vec2d(robot.position), robot.theta, to_vec2d(robot.velocity), robot.omega))
    }

    /// Simulated time, s.
    pub fn time(&self) -> f64 {
        self.sim.time()
    }

    /// Starts keeping the simulator's events for `take_events`.
    pub fn record_events(&mut self) {
        self.events.get_or_insert_with(Vec::new);
    }

    /// The events recorded since the last call (none unless `record_events`
    /// was called).
    pub fn take_events(&mut self) -> Vec<Event> {
        self.events.as_mut().map(std::mem::take).unwrap_or_default()
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
        self.clear_in_flight();
        for (&id, &(position, theta)) in &self.robot_homes {
            let _ = self.sim.teleport_robot(id, position, theta);
        }
        self.sim.teleport_ball(self.ball_home, Vec2::ZERO);
    }

    fn remove_robot(&mut self, id: RobotId) -> Result<(), ssl_sim::SimError> {
        self.sim.remove_robot(id)?;
        self.robot_homes.remove(&id);
        // A delayed frame must not bring it back.
        let engine = engine_id(id);
        for frame in &mut self.frames {
            frame.robots.retain(|(robot, ..)| *robot != engine);
        }
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
pub(crate) fn engine_id(id: RobotId) -> (i32, i32) {
    let team = match id.team {
        Team::Blue => 0,
        Team::Yellow => 1,
    };
    (id.id as i32, team)
}

fn to_sim_command(command: &RobotCommand, kick_speed: f64) -> ssl_sim::RobotCommand {
    ssl_sim::RobotCommand {
        vt: command.motion.vx.unwrap_or(0.0),
        vn: command.motion.vy.unwrap_or(0.0),
        omega: command.motion.angular.unwrap_or(0.0),
        kick_speed: if command.kicker.kick_x { kick_speed } else { 0.0 },
    }
}

fn to_vec2d(v: Vec2) -> Vec2D {
    Vec2D::new(v.x, v.y)
}

fn write_frame(frame: &Frame, world: &mut World) {
    for &((id, team), position, theta, velocity, omega) in &frame.robots {
        world.update_robot(id, team, position, theta, velocity, omega);
    }
    world.update_ball(frame.ball_velocity, frame.ball_position);
}

fn wrap_angle(a: f64) -> f64 {
    (a + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU) - std::f64::consts::PI
}

/// Gaussian samples from a seeded SplitMix64 (Box-Muller).
struct Gaussian {
    state: u64,
}

impl Gaussian {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Uniform in (0, 1]: never 0, so the log below stays finite.
    fn uniform(&mut self) -> f64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^= z >> 31;
        ((z >> 11) as f64 + 1.0) / (1u64 << 53) as f64
    }

    /// A sample with mean 0 and standard deviation `sigma` (0 without noise).
    fn sample(&mut self, sigma: f64) -> f64 {
        if sigma <= 0.0 {
            return 0.0;
        }
        let (u1, u2) = (self.uniform(), self.uniform());
        sigma * (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f64 = 1.0 / 60.0;

    fn link(edit: impl FnOnce(&mut SimConfig)) -> (SimLink, World) {
        let mut config = SimConfig::default();
        edit(&mut config);
        (SimLink::new(config), World::new(0, 0, 9.0, 6.0))
    }

    /// Blue 0 told to drive forward at 1 m/s for `ticks` ticks; its true x after.
    fn drive(link: &mut SimLink, world: &mut World, ticks: usize) -> f64 {
        let mut command = RobotCommand::new(0, 0);
        command.motion.vx = Some(1.0);
        let commands = HashMap::from([((0, 0), command)]);
        for _ in 0..ticks {
            link.apply_commands(&commands);
            link.step(DT, world);
        }
        link.sim.state().robot(RobotId::new(Team::Blue, 0)).unwrap().position.x
    }

    #[test]
    fn commands_arrive_after_the_radio_delay() {
        let (mut link, mut world) = link(|c| c.radio.delay = 0.1);
        // 0.1 s = 6 ticks: still standing at its start, x = -1.
        assert_eq!(drive(&mut link, &mut world, 6), -1.0);
        assert!(drive(&mut link, &mut world, 3) > -1.0);

        let (mut instant, mut world) = self::link(|_| {});
        assert!(drive(&mut instant, &mut world, 1) > -1.0);
    }

    #[test]
    fn vision_shows_frames_after_the_vision_delay() {
        let (mut link, mut world) = link(|c| c.vision.delay = 0.1);
        link.sim.teleport_ball(Vec2::ZERO, Vec2::new(1.0, 0.0));
        let mut truth = Vec::new();
        for _ in 0..30 {
            link.step(DT, &mut world);
            truth.push(link.sim.state().ball.position.x);
        }
        // 0.1 s = 6 ticks: the AI sees where the ball was 6 ticks ago.
        assert_eq!(world.get_ball_state().position.x, truth[truth.len() - 7]);
    }

    #[test]
    fn vision_noise_has_the_configured_spread() {
        let (mut link, mut world) = link(|c| c.vision.position_noise = 0.01);
        let mut errors = Vec::new();
        for _ in 0..2000 {
            link.step(DT, &mut world);
            let truth = link.sim.state().robot(RobotId::new(Team::Blue, 0)).unwrap().position;
            errors.push(world.get_robot_state(0, 0).position.x - truth.x);
        }
        let mean = errors.iter().sum::<f64>() / errors.len() as f64;
        let sd = (errors.iter().map(|e| (e - mean).powi(2)).sum::<f64>() / errors.len() as f64).sqrt();
        assert!(mean.abs() < 0.001, "mean {mean}");
        assert!((sd - 0.01).abs() < 0.001, "sd {sd}");
    }

    #[test]
    fn no_noise_and_no_delay_by_default() {
        let (mut link, mut world) = link(|_| {});
        link.step(DT, &mut world);
        let truth = link.sim.state().robot(RobotId::new(Team::Blue, 0)).unwrap().position;
        assert_eq!(world.get_robot_state(0, 0).position.x, truth.x);
    }
}
