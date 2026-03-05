// types.rs — Core data types replacing C++ utilities/
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Add, Mul, Sub};
use std::time::Instant;

// ─── Vec2D (replaces QVector2D) ─────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Vec2D {
    pub x: f64,
    pub y: f64,
}

impl Vec2D {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn length(&self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    pub fn length_squared(&self) -> f64 {
        self.x * self.x + self.y * self.y
    }

    pub fn normalized(&self) -> Self {
        let len = self.length();
        if len < 1e-12 {
            Self::default()
        } else {
            Self {
                x: self.x / len,
                y: self.y / len,
            }
        }
    }

    pub fn dot(&self, other: &Self) -> f64 {
        self.x * other.x + self.y * other.y
    }

    /// Rotate the vector by the given angle (radians).
    pub fn rotated(&self, angle: f64) -> Self {
        let (sin_a, cos_a) = angle.sin_cos();
        Self {
            x: cos_a * self.x - sin_a * self.y,
            y: sin_a * self.x + cos_a * self.y,
        }
    }
}

impl Add for Vec2D {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl Sub for Vec2D {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl Mul<f64> for Vec2D {
    type Output = Self;
    fn mul(self, scalar: f64) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
        }
    }
}

impl Mul<Vec2D> for f64 {
    type Output = Vec2D;
    fn mul(self, v: Vec2D) -> Vec2D {
        Vec2D {
            x: self * v.x,
            y: self * v.y,
        }
    }
}

impl fmt::Display for Vec2D {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({:.3}, {:.3})", self.x, self.y)
    }
}

// ─── RobotState ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct RobotState {
    pub id: i32,
    pub team: i32,
    pub position: Vec2D,
    pub velocity: Vec2D,
    pub commanded_velocity: Vec2D,
    pub commanded_angular: f64,
    pub angular_velocity: f64,
    pub orientation: f64,
    pub active: bool,
    pub last_update: Instant,
}

impl Default for RobotState {
    fn default() -> Self {
        Self {
            id: 0,
            team: 0,
            position: Vec2D::default(),
            velocity: Vec2D::default(),
            commanded_velocity: Vec2D::default(),
            commanded_angular: 0.0,
            angular_velocity: 0.0,
            orientation: 0.0,
            active: false,
            last_update: Instant::now(),
        }
    }
}



impl RobotState {
    pub fn new(id: i32, team: i32) -> Self {
        Self {
            id,
            team,
            ..Default::default()
        }
    }
}

// ─── BallState ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct BallState {
    pub position: Vec2D,
    pub velocity: Vec2D,
}

impl Default for BallState {
    fn default() -> Self {
        Self {
            position: Vec2D::default(),
            velocity: Vec2D::default(),
        }
    }
}

impl BallState {
    pub fn is_moving(&self) -> bool {
        self.velocity.length() > 0.01
    }
}

// ─── MotionCommand ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub struct MotionCommand {
    pub id: i32,
    pub team: i32,
    pub vx: f64,
    pub vy: f64,
    pub angular: f64,
}

impl MotionCommand {
    pub fn new(id: i32, team: i32, vx: f64, vy: f64) -> Self {
        Self {
            id,
            team,
            vx,
            vy,
            angular: 0.0,
        }
    }

    pub fn with_id_team(id: i32, team: i32) -> Self {
        Self::new(id, team, 0.0, 0.0)
    }

    pub fn zero() -> Self {
        Self::new(0, 0, 0.0, 0.0)
    }
}

impl fmt::Display for MotionCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MotionCmd(id={}, team={}, vx={:.2}, vy={:.2}, w={:.2})",
            self.id, self.team, self.vx, self.vy, self.angular
        )
    }
}

// ─── KickerCommand ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub struct KickerCommand {
    pub id: i32,
    pub team: i32,
    pub kick_x: bool,
    pub kick_z: bool,
    pub dribbler: f64,
}

impl KickerCommand {
    pub fn new(id: i32, team: i32) -> Self {
        Self {
            id,
            team,
            kick_x: false,
            kick_z: false,
            dribbler: 0.0,
        }
    }
}

// ─── RobotCommand ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct RobotCommand {
    pub id: i32,
    pub team: i32,
    pub motion: MotionCommand,
    pub kicker: KickerCommand,
}

impl RobotCommand {
    pub fn new(id: i32, team: i32) -> Self {
        Self {
            id,
            team,
            motion: MotionCommand::with_id_team(id, team),
            kicker: KickerCommand::new(id, team),
        }
    }
}

// ─── PathTestState ──────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PathTestState {
    pub id: i32,
    pub team: i32,
    pub controller: String,
    pub params: crate::ControllerParams,
    pub points: Vec<Vec2D>,
    pub current_target_idx: usize,
}

impl PathTestState {
    pub fn new(id: i32, team: i32, controller: String, params: crate::ControllerParams, points: Vec<Vec2D>) -> Self {
        Self {
            id,
            team,
            controller,
            params,
            points,
            current_target_idx: 0,
        }
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vec2d_length() {
        let v = Vec2D::new(3.0, 4.0);
        assert!((v.length() - 5.0).abs() < 1e-9);
    }

    #[test]
    fn vec2d_normalize() {
        let v = Vec2D::new(3.0, 4.0);
        let n = v.normalized();
        assert!((n.length() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn vec2d_ops() {
        let a = Vec2D::new(1.0, 2.0);
        let b = Vec2D::new(3.0, 4.0);
        let sum = a + b;
        assert!((sum.x - 4.0).abs() < 1e-9);
        assert!((sum.y - 6.0).abs() < 1e-9);

        let diff = b - a;
        assert!((diff.x - 2.0).abs() < 1e-9);
        assert!((diff.y - 2.0).abs() < 1e-9);

        let scaled = a * 3.0;
        assert!((scaled.x - 3.0).abs() < 1e-9);
        assert!((scaled.y - 6.0).abs() < 1e-9);
    }

    #[test]
    fn vec2d_rotate() {
        let v = Vec2D::new(1.0, 0.0);
        let rotated = v.rotated(std::f64::consts::FRAC_PI_2);
        assert!(rotated.x.abs() < 1e-9);
        assert!((rotated.y - 1.0).abs() < 1e-9);
    }

    #[test]
    fn robot_state_default_inactive() {
        let r = RobotState::default();
        assert!(!r.active);
    }

    #[test]
    fn ball_state_not_moving() {
        let b = BallState::default();
        assert!(!b.is_moving());
    }

    #[test]
    fn motion_command_display() {
        let cmd = MotionCommand::new(1, 0, 1.5, -0.5);
        let s = format!("{cmd}");
        assert!(s.contains("id=1"));
    }
}
