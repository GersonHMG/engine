// motion/traction.rs — Keeps every robot's velocity commands within its grip
//
// Whatever sends the command (move_to, send_velocity, manual control), the
// robots get it through here, once per tick, right before it goes out: the
// commanded velocity, in field coordinates, changes by at most
// `ramp_accel · dt` per tick (`ramp_accel` from `motion::settings`).
//
// Without it a new target makes the command jump (the bang-bang plan
// commands its cruise speed at once, or 0 to brake) and the motors spin the
// wheels as hard as they can: more than the floor's grip, so the robot
// slides, overshoots and loses its heading. Ramping in field coordinates also
// means a robot that turns while it drives keeps its velocity instead of
// swinging it around with the body.

use crate::types::{RobotState, Vec2D};
use std::collections::HashMap;

/// Engine tick, s: how often commands go out.
pub const TICK: f64 = 1.0 / 60.0;

/// The velocity to command this tick, in field coordinates: `wanted`, moved
/// at most `ramp_accel · dt` away from `from` (what was commanded last tick,
/// or what vision measures for a robot that had no command). `ramp_accel`
/// 0 = `wanted` as it is.
pub fn shape(from: Vec2D, wanted: Vec2D, ramp_accel: f64, dt: f64) -> Vec2D {
    if ramp_accel <= 0.0 {
        return wanted;
    }
    let change = wanted - from;
    let max = ramp_accel * dt;
    let length = change.length();
    if length > max { from + change * (max / length) } else { wanted }
}

/// The velocity each robot was commanded last tick, to ramp from.
#[derive(Debug, Default)]
pub struct TractionControl {
    last: HashMap<(i32, i32), Vec2D>,
}

impl TractionControl {
    /// Ramps robot (id, team)'s command `(vx, vy)` (robot frame, m/s) and
    /// returns what to send instead. A robot vision does not see is passed
    /// through unchanged.
    pub fn limit(&mut self, robot: &RobotState, vx: f64, vy: f64, ramp_accel: f64) -> (f64, f64) {
        let key = (robot.id, robot.team);
        if !robot.active {
            self.last.remove(&key);
            return (vx, vy);
        }
        let wanted = Vec2D::new(vx, vy).rotated(robot.orientation);
        let from = self.last.get(&key).copied().unwrap_or(robot.velocity);
        let sent = shape(from, wanted, ramp_accel, TICK);
        self.last.insert(key, sent);
        let local = sent.rotated(-robot.orientation);
        (local.x, local.y)
    }

    /// Only the robots in `commanded` got a command this tick: the others
    /// start from their measured velocity next time.
    pub fn keep_only(&mut self, commanded: impl Fn(&(i32, i32)) -> bool) {
        self.last.retain(|key, _| commanded(key));
    }

    /// The robot was moved by hand: start again from what vision sees.
    pub fn forget(&mut self, id: i32, team: i32) {
        self.last.remove(&(id, team));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Vec2D, b: Vec2D) -> bool {
        (a - b).length() < 1e-9
    }

    fn seen(orientation: f64, velocity: Vec2D) -> RobotState {
        let mut robot = RobotState::new(0, 0);
        robot.active = true;
        robot.orientation = orientation;
        robot.velocity = velocity;
        robot
    }

    #[test]
    fn a_step_in_the_command_becomes_a_ramp_both_ways() {
        let mut v = Vec2D::new(0.0, 0.0);
        for tick in 1..=30 {
            v = shape(v, Vec2D::new(3.0, 0.0), 2.0, TICK);
            assert!(close(v, Vec2D::new(2.0 * TICK * tick as f64, 0.0)), "tick {tick}: {v}");
        }
        for _ in 0..30 {
            v = shape(v, Vec2D::new(0.0, 0.0), 2.0, TICK);
        }
        assert!(close(v, Vec2D::new(0.0, 0.0)), "braking ramps down to 0, got {v}");
    }

    #[test]
    fn zero_ramp_passes_the_command_through() {
        let wanted = Vec2D::new(-2.0, 1.0);
        assert!(close(shape(Vec2D::new(3.0, 0.0), wanted, 0.0, TICK), wanted));
    }

    #[test]
    fn the_ramp_is_in_field_coordinates() {
        // Facing +y and moving +y at 1 m/s, "forward 1 m/s" is already reached; then the
        // robot turns to face +x and still asks for field +y, i.e. "right" in its frame.
        let mut control = TractionControl::default();
        let (vx, vy) = control.limit(&seen(std::f64::consts::FRAC_PI_2, Vec2D::new(0.0, 1.0)), 1.0, 0.0, 2.0);
        assert!((vx - 1.0).abs() < 1e-9 && vy.abs() < 1e-9, "({vx}, {vy})");
        let (vx, vy) = control.limit(&seen(0.0, Vec2D::new(0.0, 1.0)), 0.0, 1.0, 2.0);
        assert!(vx.abs() < 1e-9 && (vy - 1.0).abs() < 1e-9, "({vx}, {vy})");
    }

    #[test]
    fn a_robot_without_a_command_starts_from_its_measured_velocity() {
        let mut control = TractionControl::default();
        let robot = seen(0.0, Vec2D::new(1.0, 0.0));
        control.limit(&robot, 3.0, 0.0, 2.0);
        control.keep_only(|_| false);
        let (vx, _) = control.limit(&robot, 3.0, 0.0, 2.0);
        assert!((vx - (1.0 + 2.0 * TICK)).abs() < 1e-9, "{vx}");
    }
}
