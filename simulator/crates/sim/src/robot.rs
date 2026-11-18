use crate::config::RobotConfig;
use crate::math::{wrap_angle, Vec2};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Slip speed past which the wheels let go and the robot slides, m/s.
const SLIDE_SLIP: f64 = 0.1;

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
    /// Ground speed of the wheels in the robot's frame (what the motors
    /// drive), m/s. Differs from `velocity` only while the robot slides.
    #[serde(default)]
    pub wheel_velocity: Vec2,
    /// The wheels are slipping on the floor (see `RobotConfig::grip_accel`).
    #[serde(default)]
    pub sliding: bool,
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
            wheel_velocity: Vec2::ZERO,
            sliding: false,
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
    ///
    /// With `grip_accel` > 0 the motors drive the wheels (in the robot's
    /// frame) and the floor drags the body after them with at most
    /// `grip_accel`: asking for more makes the wheels slip and the robot
    /// slide (see `RobotConfig`).
    pub(crate) fn drive(&mut self, config: &RobotConfig, dt: f64) {
        if config.grip_accel > 0.0 {
            self.drive_on_wheels(config, dt);
        } else {
            let (local, omega) = self.wheel_command(config);
            let target = local.rotated(self.theta);
            self.velocity += (target - self.velocity).clamp_length(config.max_accel * dt);
            self.sliding = false;
            self.turn(config, dt, omega, 0.0);
            self.wheel_velocity = self.velocity.rotated(-self.theta);
        }
        self.position += self.velocity * dt;
    }

    /// Slip model: wheels follow the command, the body follows the wheels.
    fn drive_on_wheels(&mut self, config: &RobotConfig, dt: f64) {
        let (target, omega) = self.wheel_command(config);
        self.wheel_velocity += (target - self.wheel_velocity).clamp_length(config.max_accel * dt);

        let slip = self.wheel_velocity.rotated(self.theta) - self.velocity;
        let grip = config.grip_accel * if self.sliding { config.slide_grip } else { 1.0 };
        let step = grip * dt;
        let slip_speed = slip.length();
        let left = (slip_speed - step).max(0.0);
        if left == 0.0 {
            self.velocity += slip;
            self.sliding = false;
        } else {
            self.velocity += slip * (step / slip_speed);
            // Tyres take small slips (a command that steps every tick)
            // without letting go; a robot that falls further behind slides.
            if left > SLIDE_SLIP {
                self.sliding = true;
            }
        }
        let spin = if self.sliding { config.slip_spin * left } else { 0.0 };
        self.turn(config, dt, omega, spin);
    }

    /// The commanded velocity (robot frame) and angular speed within
    /// `max_speed`, `max_omega` and the wheels' top speed.
    fn wheel_command(&self, config: &RobotConfig) -> (Vec2, f64) {
        let local = Vec2::new(self.command.vt, self.command.vn).clamp_length(config.max_speed);
        let omega = self.command.omega.clamp(-config.max_omega, config.max_omega);
        config.wheel_limited(local, omega)
    }

    /// Angular speed toward `target_omega` within `max_alpha`; `extra`
    /// (rad/s) turns the heading on top of it.
    fn turn(&mut self, config: &RobotConfig, dt: f64, target_omega: f64, extra: f64) {
        let max_step = config.max_alpha * dt;
        self.omega += (target_omega - self.omega).clamp(-max_step, max_step);
        self.theta = wrap_angle(self.theta + (self.omega + extra) * dt);
    }
}
