use serde::{Deserialize, Serialize};
use std::f64::consts::PI;
use std::ops::{Add, AddAssign, Mul, Neg, Sub, SubAssign};

/// Lengths below this are treated as zero.
pub(crate) const EPSILON: f64 = 1e-12;

/// 2D vector.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

impl Vec2 {
    pub const ZERO: Vec2 = Vec2 { x: 0.0, y: 0.0 };

    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn dot(self, other: Vec2) -> f64 {
        self.x * other.x + self.y * other.y
    }

    pub fn length(self) -> f64 {
        self.dot(self).sqrt()
    }

    /// Unit vector in the same direction, or `None` for a zero vector.
    pub fn normalized(self) -> Option<Vec2> {
        let length = self.length();
        (length > EPSILON).then(|| self * (1.0 / length))
    }

    /// Rotated by +90°.
    pub fn perp(self) -> Vec2 {
        Vec2::new(-self.y, self.x)
    }

    /// Rotated counter-clockwise by `angle` radians.
    pub fn rotated(self, angle: f64) -> Vec2 {
        let (sin, cos) = (libm::sin(angle), libm::cos(angle));
        Vec2::new(cos * self.x - sin * self.y, sin * self.x + cos * self.y)
    }

    /// Same direction, length capped at `max`.
    pub fn clamp_length(self, max: f64) -> Vec2 {
        let length = self.length();
        if length > max {
            self * (max / length)
        } else {
            self
        }
    }
}

impl Add for Vec2 {
    type Output = Vec2;
    fn add(self, other: Vec2) -> Vec2 {
        Vec2::new(self.x + other.x, self.y + other.y)
    }
}

impl Sub for Vec2 {
    type Output = Vec2;
    fn sub(self, other: Vec2) -> Vec2 {
        Vec2::new(self.x - other.x, self.y - other.y)
    }
}

impl Mul<f64> for Vec2 {
    type Output = Vec2;
    fn mul(self, k: f64) -> Vec2 {
        Vec2::new(self.x * k, self.y * k)
    }
}

impl Neg for Vec2 {
    type Output = Vec2;
    fn neg(self) -> Vec2 {
        Vec2::new(-self.x, -self.y)
    }
}

impl AddAssign for Vec2 {
    fn add_assign(&mut self, other: Vec2) {
        *self = *self + other;
    }
}

impl SubAssign for Vec2 {
    fn sub_assign(&mut self, other: Vec2) {
        *self = *self - other;
    }
}

/// Wraps an angle to (-π, π].
pub fn wrap_angle(mut angle: f64) -> f64 {
    while angle > PI {
        angle -= 2.0 * PI;
    }
    while angle <= -PI {
        angle += 2.0 * PI;
    }
    angle
}
