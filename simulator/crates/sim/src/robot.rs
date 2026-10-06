use crate::config::RobotConfig;
use crate::math::{wrap_angle, Vec2};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Team {
    Blue,
    Yellow,
}

/// A robot is identified by its team and its number within the team.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RobotId {
    pub team: Team,
    pub id: u32,
}

impl RobotId {
    pub const fn new(team: Team, id: u32) -> Self {
        Self { team, id }
    }
}

impl fmt::Display for RobotId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let team = match self.team {
            Team::Blue => "blue",
            Team::Yellow => "yellow",
        };
        write!(f, "{team} {}", self.id)
    }
}

/// What the robot is told to do, in its own frame (grSim convention).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RobotCommand {
    /// Forward velocity, m/s.
    pub vt: f64,
    /// Leftward velocity, m/s.
    pub vn: f64,
    /// Angular velocity, rad/s, counter-clockwise.
    pub omega: f64,
    /// Kick speed, m/s. Fires once, as soon as the ball is at the kicker;
    /// 0 = no kick.
    pub kick_speed: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Robot {
    pub id: RobotId,
    pub position: Vec2,
    /// Heading, rad; 0 = facing +x.
    pub theta: f64,
    pub velocity: Vec2,
    /// rad/s.
    pub omega: f64,
    pub command: RobotCommand,
    /// Contacts in progress, so contact events fire once per contact.
    pub(crate) touching_ball: bool,
    pub(crate) touching_wall: bool,
}

impl Robot {
    /// A robot at rest.
    pub fn new(id: RobotId, position: Vec2, theta: f64) -> Self {
        Self {
            id,
            position,
            theta: wrap_angle(theta),
            velocity: Vec2::ZERO,
            omega: 0.0,
            command: RobotCommand::default(),
            touching_ball: false,
            touching_wall: false,
        }
    }

    /// Unit vector the robot faces.
    pub fn heading(&self) -> Vec2 {
        Vec2::new(1.0, 0.0).rotated(self.theta)
    }

    /// A world point in the robot's frame.
    pub fn to_local(&self, point: Vec2) -> Vec2 {
        (point - self.position).rotated(-self.theta)
    }

    /// Velocity of the body at a world point, including rotation.
    pub fn point_velocity(&self, point: Vec2) -> Vec2 {
        self.velocity + (point - self.position).perp() * self.omega
    }

    /// Tracks the commanded velocities within the acceleration limits, then
    /// integrates the pose (semi-implicit Euler).
    pub(crate) fn drive(&mut self, config: &RobotConfig, dt: f64) {
        let target = Vec2::new(self.command.vt, self.command.vn)
            .rotated(self.theta)
            .clamp_length(config.max_speed);
        self.velocity += (target - self.velocity).clamp_length(config.max_accel * dt);

        let max_step = config.max_alpha * dt;
        let target_omega = self.command.omega.clamp(-config.max_omega, config.max_omega);
        self.omega += (target_omega - self.omega).clamp(-max_step, max_step);

        self.position += self.velocity * dt;
        self.theta = wrap_angle(self.theta + self.omega * dt);
    }
}
