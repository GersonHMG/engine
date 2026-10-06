use crate::config::BallConfig;
use crate::math::Vec2;
use serde::{Deserialize, Serialize};

/// A solid sphere hit at its center slides until it has slowed to 5/7 of
/// its speed, then rolls without slipping.
const ROLLING_RATIO: f64 = 5.0 / 7.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ball {
    pub position: Vec2,
    pub velocity: Vec2,
    /// Speed below which the ball rolls instead of sliding.
    rolling_speed: f64,
}

impl Ball {
    /// A ball just given `velocity` (sliding).
    pub fn new(position: Vec2, velocity: Vec2) -> Self {
        let mut ball = Self {
            position,
            velocity: Vec2::ZERO,
            rolling_speed: 0.0,
        };
        ball.set_velocity(velocity);
        ball
    }

    pub fn is_sliding(&self) -> bool {
        self.velocity.length() > self.rolling_speed
    }

    /// Velocity after an impulse (kick, bounce): the ball slides again.
    pub(crate) fn set_velocity(&mut self, velocity: Vec2) {
        self.velocity = velocity;
        self.rolling_speed = ROLLING_RATIO * velocity.length();
    }

    /// Applies sliding or rolling friction, then integrates the position.
    pub(crate) fn roll(&mut self, config: &BallConfig, dt: f64) {
        let speed = self.velocity.length();
        if speed > 0.0 {
            let decel = if speed > self.rolling_speed {
                config.slide_decel
            } else {
                config.roll_decel
            };
            let new_speed = speed - decel * dt;
            self.velocity = if new_speed > 0.0 {
                self.velocity * (new_speed / speed)
            } else {
                Vec2::ZERO
            };
        }
        self.position += self.velocity * dt;
    }
}
