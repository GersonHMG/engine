//! Deterministic 2D physics for RoboCup SSL.
//!
//! Scope: robots of two teams, one ball and a field closed by walls.
//!
//! Conventions:
//! * SI units: meters, seconds, radians.
//! * Field frame: origin at the center, x along the field length.
//! * Robot frame: x forward (kicker side), y to the left. Commands use this
//!   frame like grSim (`vt` = forward, `vn` = left).
//!
//! The simulator has no I/O, no globals and no randomness, and uses `libm`
//! for trigonometry, so the same inputs give the same bits on every platform.

mod ball;
mod collision;
mod config;
mod event;
mod field;
mod math;
mod robot;
mod simulator;

pub use ball::Ball;
pub use config::{BallConfig, FieldConfig, RobotConfig, SimConfig};
pub use event::{Event, EventKind};
pub use math::{wrap_angle, Vec2};
pub use robot::{Robot, RobotCommand, RobotId, Team};
pub use simulator::{SimError, SimState, Simulator};
