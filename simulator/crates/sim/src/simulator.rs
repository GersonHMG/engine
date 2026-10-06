use crate::ball::Ball;
use crate::collision;
use crate::config::SimConfig;
use crate::event::{Event, EventKind};
use crate::field;
use crate::math::Vec2;
use crate::robot::{Robot, RobotCommand, RobotId, Team};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Everything that evolves over time. Cloning it is a full snapshot:
/// `Simulator::restore` with a clone replays exactly the same trajectory.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimState {
    /// Simulation time, s.
    pub time: f64,
    /// Sorted by id, ids unique.
    robots: Vec<Robot>,
    pub ball: Ball,
}

impl SimState {
    /// Robots with a repeated id are dropped (the first one is kept).
    pub fn new(mut robots: Vec<Robot>, ball: Ball) -> Self {
        robots.sort_by_key(|robot| robot.id);
        robots.dedup_by_key(|robot| robot.id);
        Self { time: 0.0, robots, ball }
    }

    /// Sorted by id.
    pub fn robots(&self) -> &[Robot] {
        &self.robots
    }

    pub fn robot(&self, id: RobotId) -> Option<&Robot> {
        self.index_of(id).ok().map(|i| &self.robots[i])
    }

    fn index_of(&self, id: RobotId) -> Result<usize, usize> {
        self.robots.binary_search_by_key(&id, |robot| robot.id)
    }
}

impl Default for SimState {
    /// Blue robot 0, 1 m behind the ball and facing it; ball at rest at the center.
    fn default() -> Self {
        Self::new(
            vec![Robot::new(RobotId::new(Team::Blue, 0), Vec2::new(-1.0, 0.0), 0.0)],
            Ball::new(Vec2::ZERO, Vec2::ZERO),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimError {
    UnknownRobot(RobotId),
    DuplicateRobot(RobotId),
}

impl fmt::Display for SimError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SimError::UnknownRobot(id) => write!(f, "there is no robot {id}"),
            SimError::DuplicateRobot(id) => write!(f, "robot {id} already exists"),
        }
    }
}

impl std::error::Error for SimError {}

pub struct Simulator {
    config: SimConfig,
    state: SimState,
    events: Vec<Event>,
}

impl Simulator {
    pub fn new(config: SimConfig, state: SimState) -> Self {
        Self {
            config,
            state,
            events: Vec::new(),
        }
    }

    pub fn config(&self) -> &SimConfig {
        &self.config
    }

    /// Takes effect from the next step.
    pub fn set_config(&mut self, config: SimConfig) {
        self.config = config;
    }

    pub fn state(&self) -> &SimState {
        &self.state
    }

    pub fn time(&self) -> f64 {
        self.state.time
    }

    /// Adds a robot at rest.
    pub fn add_robot(&mut self, id: RobotId, position: Vec2, theta: f64) -> Result<(), SimError> {
        match self.state.index_of(id) {
            Ok(_) => Err(SimError::DuplicateRobot(id)),
            Err(index) => {
                self.state.robots.insert(index, Robot::new(id, position, theta));
                Ok(())
            }
        }
    }

    pub fn remove_robot(&mut self, id: RobotId) -> Result<(), SimError> {
        let index = self.state.index_of(id).map_err(|_| SimError::UnknownRobot(id))?;
        self.state.robots.remove(index);
        Ok(())
    }

    /// Replaces a robot's command; it stays in effect until the next call.
    pub fn set_command(&mut self, id: RobotId, command: RobotCommand) -> Result<(), SimError> {
        self.robot_mut(id)?.command = command;
        Ok(())
    }

    /// Places a robot at rest, keeping its command.
    pub fn teleport_robot(&mut self, id: RobotId, position: Vec2, theta: f64) -> Result<(), SimError> {
        let robot = self.robot_mut(id)?;
        let command = robot.command;
        *robot = Robot::new(id, position, theta);
        robot.command = command;
        Ok(())
    }

    pub fn teleport_ball(&mut self, position: Vec2, velocity: Vec2) {
        self.state.ball = Ball::new(position, velocity);
    }

    /// Goes back to a snapshot taken with `state().clone()`.
    pub fn restore(&mut self, state: SimState) {
        self.state = state;
        self.events.clear();
    }

    /// Events since the last call, oldest first.
    pub fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    /// Advances the simulation by `dt` seconds, in equal substeps no longer
    /// than `config.max_substep`.
    pub fn step(&mut self, dt: f64) {
        assert!(dt >= 0.0 && dt.is_finite(), "step dt must be finite and >= 0, got {dt}");
        let substeps = (dt / self.config.max_substep).ceil().max(1.0) as usize;
        let h = dt / substeps as f64;
        for _ in 0..substeps {
            self.substep(h);
        }
    }

    fn robot_mut(&mut self, id: RobotId) -> Result<&mut Robot, SimError> {
        let index = self.state.index_of(id).map_err(|_| SimError::UnknownRobot(id))?;
        Ok(&mut self.state.robots[index])
    }

    fn substep(&mut self, dt: f64) {
        self.state.time += dt;
        self.move_robots(dt);
        self.collide_robots();
        self.move_ball(dt);
        for index in 0..self.state.robots.len() {
            self.collide_robot_ball(index);
            self.kick(index);
        }
    }

    /// Drives every robot; walls stop them.
    fn move_robots(&mut self, dt: f64) {
        let time = self.state.time;
        for robot in &mut self.state.robots {
            robot.drive(&self.config.robot, dt);

            let walls = field::confine(&self.config.field, &mut robot.position, self.config.robot.radius);
            walls.apply(&mut robot.velocity, |_| 0.0);

            let touching = walls.any();
            if touching && !robot.touching_wall {
                let kind = EventKind::RobotHitWall { robot: robot.id, position: robot.position };
                self.events.push(Event { time, kind });
            }
            robot.touching_wall = touching;
        }
    }

    /// Pushes overlapping robots apart (equal masses) and stops them from
    /// moving into each other.
    fn collide_robots(&mut self) {
        let min_distance = 2.0 * self.config.robot.radius;
        let robots = &mut self.state.robots;

        for j in 1..robots.len() {
            let (before, rest) = robots.split_at_mut(j);
            let b = &mut rest[0];
            for a in before.iter_mut() {
                let offset = b.position - a.position;
                let overlap = min_distance - offset.length();
                if overlap <= 0.0 {
                    continue;
                }

                let normal = offset.normalized().unwrap_or(Vec2::new(1.0, 0.0));
                a.position -= normal * (overlap / 2.0);
                b.position += normal * (overlap / 2.0);

                let closing = (b.velocity - a.velocity).dot(normal);
                if closing < 0.0 {
                    a.velocity += normal * (closing / 2.0);
                    b.velocity -= normal * (closing / 2.0);
                }
            }
        }
    }

    /// Rolls the ball; walls bounce it.
    fn move_ball(&mut self, dt: f64) {
        let ball = &mut self.state.ball;
        ball.roll(&self.config.ball, dt);

        let walls = field::confine(&self.config.field, &mut ball.position, self.config.ball.radius);
        let restitution = self.config.ball.wall_restitution;
        let mut velocity = ball.velocity;
        if walls.apply(&mut velocity, |v| -restitution * v) {
            let speed = ball.velocity.length();
            ball.set_velocity(velocity);
            let kind = EventKind::BallHitWall { position: ball.position, speed };
            self.events.push(Event { time: self.state.time, kind });
        }
    }

    /// Pushes the ball out of a robot and bounces it off the body. Robots
    /// are far heavier than the ball, so they are not affected.
    fn collide_robot_ball(&mut self, index: usize) {
        let robot = &mut self.state.robots[index];
        let ball = &mut self.state.ball;
        let local = robot.to_local(ball.position);

        let Some(contact) = collision::robot_ball(&self.config.robot, local, self.config.ball.radius) else {
            robot.touching_ball = false;
            return;
        };

        let normal = contact.normal.rotated(robot.theta);
        let point = robot.position + contact.point.rotated(robot.theta);
        ball.position += normal * contact.depth;

        let restitution = if contact.on_front {
            self.config.robot.front_restitution
        } else {
            self.config.robot.body_restitution
        };
        let approach = (ball.velocity - robot.point_velocity(point)).dot(normal);
        if approach < 0.0 {
            ball.set_velocity(ball.velocity - normal * ((1.0 + restitution) * approach));
        }

        if !robot.touching_ball {
            robot.touching_ball = true;
            let kind = EventKind::RobotTouchedBall { robot: robot.id, on_front: contact.on_front };
            self.events.push(Event { time: self.state.time, kind });
        }
    }

    /// Fires a robot's pending kick when the ball is at its front face. One
    /// kick per command.
    fn kick(&mut self, index: usize) {
        let config = &self.config.robot;
        let robot = &mut self.state.robots[index];
        let ball = &mut self.state.ball;

        let speed = robot.command.kick_speed.min(config.max_kick_speed);
        if speed <= 0.0 {
            return;
        }

        let local = robot.to_local(ball.position);
        let reach = config.front_distance + self.config.ball.radius + config.kick_reach;
        let at_kicker = local.x >= config.front_distance
            && local.x <= reach
            && local.y.abs() <= config.front_half_width();
        if !at_kicker {
            return;
        }

        ball.set_velocity(robot.velocity + robot.heading() * speed);
        robot.command.kick_speed = 0.0;
        let kind = EventKind::Kick { robot: robot.id, speed };
        self.events.push(Event { time: self.state.time, kind });
    }
}
