use crate::math::Vec2;
use crate::robot::RobotId;
use serde::Serialize;

/// Something notable that happened during a step, for feedback to agents.
/// Contact events fire when the contact starts, not on every substep.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Event {
    /// Simulation time, s.
    pub time: f64,
    #[serde(flatten)]
    pub kind: EventKind,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EventKind {
    /// The ball bounced off a wall; `speed` is its speed before the bounce.
    BallHitWall { position: Vec2, speed: f64 },
    /// A robot ran into a wall.
    RobotHitWall { robot: RobotId, position: Vec2 },
    /// A robot started touching the ball.
    RobotTouchedBall { robot: RobotId, on_front: bool },
    /// A robot kicked the ball.
    Kick { robot: RobotId, speed: f64 },
}
