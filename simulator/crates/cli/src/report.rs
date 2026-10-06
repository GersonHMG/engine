//! The JSON result of a run: final state plus everything that happened.

use serde::Serialize;
use ssl_sim::{Event, RobotId, SimState, Vec2};

#[derive(Serialize)]
pub struct Report {
    pub scenario: String,
    /// Simulated time, s.
    pub duration: f64,
    pub ticks: usize,
    #[serde(rename = "final")]
    pub final_state: FinalState,
    pub events: Vec<Event>,
}

#[derive(Serialize)]
pub struct FinalState {
    pub robots: Vec<RobotView>,
    pub ball: BallView,
}

#[derive(Serialize)]
pub struct RobotView {
    pub id: RobotId,
    pub position: Vec2,
    pub theta: f64,
    pub velocity: Vec2,
    pub omega: f64,
}

#[derive(Serialize)]
pub struct BallView {
    pub position: Vec2,
    pub velocity: Vec2,
    pub speed: f64,
    pub sliding: bool,
}

impl FinalState {
    pub fn from_state(state: &SimState) -> Self {
        let ball = &state.ball;
        Self {
            robots: state
                .robots()
                .iter()
                .map(|robot| RobotView {
                    id: robot.id,
                    position: robot.position,
                    theta: robot.theta,
                    velocity: robot.velocity,
                    omega: robot.omega,
                })
                .collect(),
            ball: BallView {
                position: ball.position,
                velocity: ball.velocity,
                speed: ball.velocity.length(),
                sliding: ball.is_sliding(),
            },
        }
    }
}
