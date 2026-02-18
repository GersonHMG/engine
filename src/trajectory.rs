// trajectory.rs — Trajectory planning (BangBang control)
// Port of motion/bangbangcontrol/trajectory1d.cpp, trajectory2d.cpp, bangbangcontrol.cpp

use crate::types::{MotionCommand, RobotState, Vec2D};

// ─── Trajectory1D ───────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
struct State {
    v: f64,  // Final velocity
    t: f64,  // Final time
    _a: f64, // Acceleration applied
    _d: f64, // Distance traveled
}

impl Default for State {
    fn default() -> Self {
        Self {
            v: 0.0,
            t: 0.0,
            _a: 0.0,
            _d: 0.0,
        }
    }
}

struct Constraints {
    a_max: f64,
    v_max: f64,
}

// Case implementations
fn case1(last: &State, c: &Constraints) -> State {
    let t = -last.v / c.a_max;
    let d = -(last.v * last.v) / (2.0 * c.a_max);
    State { v: 0.0, t, _a: c.a_max, _d: d }
}

fn case2_1(last: &State, c: &Constraints, wf: f64) -> State {
    let t_i = (c.v_max - last.v) / c.a_max;
    let v1 = (wf * c.a_max + (last.v * last.v) / 2.0).sqrt();
    let t_ii = (v1 - last.v) / c.a_max;

    if t_i < t_ii {
        let d = (c.v_max * c.v_max - last.v * last.v) / (2.0 * c.a_max);
        State { v: c.v_max, t: t_i, _a: c.a_max, _d: d }
    } else {
        let d = wf / 2.0 + (last.v * last.v) / (2.0 * c.a_max);
        State { v: v1, t: t_ii, _a: c.a_max, _d: d }
    }
}

fn case2_2(last: &State, c: &Constraints, wf: f64) -> State {
    let t = (wf / c.v_max) - (c.v_max / (2.0 * c.a_max));
    let d = wf - ((last.v * last.v) / (2.0 * c.a_max));
    State { v: c.v_max, t, _a: 0.0, _d: d }
}

fn case2_3(last: &State, c: &Constraints) -> State {
    let t = last.v / c.a_max;
    let d = (last.v * last.v) / (2.0 * c.a_max);
    State { v: 0.0, t, _a: -c.a_max, _d: d }
}

fn case3(last: &State, c: &Constraints) -> State {
    let t = (last.v - c.v_max) / c.a_max;
    let d = (1.0 / (2.0 * c.a_max)) * (last.v * last.v - c.v_max * c.v_max);
    State { v: c.v_max, t, _a: -c.a_max, _d: d }
}

#[derive(Clone, Debug)]
pub struct Trajectory1D {
    states: Vec<State>,
}

impl Default for Trajectory1D {
    fn default() -> Self {
        Self { states: Vec::new() }
    }
}

impl Trajectory1D {
    pub fn new(a_max: f64, v_max: f64, v0: f64, wf: f64) -> Self {
        assert!(a_max > 0.0 && v_max > 0.0, "Max acceleration and velocity must be > 0");

        if wf == 0.0 {
            return Self {
                states: vec![State { v: v0, t: 0.0, _a: 0.0, _d: 0.0 }],
            };
        }

        // Normalize
        let wf_sign: f64 = if wf < 0.0 { -1.0 } else { 1.0 };
        let wf = wf_sign * wf;
        let v0 = wf_sign * v0;

        let state_0 = State { v: v0, t: 0.0, _a: 0.0, _d: 0.0 };
        let c = Constraints { a_max, v_max };
        let mut states = vec![state_0.clone()];

        if v0 < 0.0 {
            states.push(case1(&state_0, &c));
        } else if wf > (v0 * v0) / (2.0 * a_max) && v_max > v0 && v0 >= 0.0 {
            states.push(case2_1(&state_0, &c, wf));
        } else if wf > (v0 * v0) / (2.0 * a_max) && (v_max - v0).abs() <= 0.002 {
            states.push(case2_2(&state_0, &c, wf));
        } else if wf <= (v0 * v0) / (2.0 * a_max) && 0.0 < v0 && v0 <= v_max {
            states.push(case2_3(&state_0, &c));
        } else if v0 > v_max {
            states.push(case3(&state_0, &c));
        }

        // Denormalize if needed
        if wf_sign == -1.0 {
            for s in &mut states {
                s.v = -s.v;
            }
        }

        Self { states }
    }

    pub fn tf(&self) -> f64 {
        self.states.last().map(|s| s.t).unwrap_or(0.0)
    }

    /// Returns (time, velocity) as the solution.
    pub fn get_solution(&self) -> (f64, f64) {
        self.states.last().map(|s| (s.t, s.v)).unwrap_or((0.0, 0.0))
    }
}

// ─── Trajectory2D ───────────────────────────────────────────────────────────

pub struct Trajectory2D {
    traj_x: Trajectory1D,
    traj_y: Trajectory1D,
    valid: bool,
}

impl Trajectory2D {
    pub fn new(a_max: f64, v_max: f64, v0: Vec2D, from: Vec2D, to: Vec2D) -> Self {
        let wfx = to.x - from.x;
        let wfy = to.y - from.y;

        let mut min_alpha: f64 = 0.0;
        let mut max_alpha = std::f64::consts::FRAC_PI_2;
        let epsilon = 0.05;

        let mut traj_x = Trajectory1D::default();
        let mut traj_y = Trajectory1D::default();
        let mut valid = false;

        for _ in 0..20 {
            let mid_alpha = (min_alpha + max_alpha) / 2.0;

            let tx = Trajectory1D::new(
                a_max * mid_alpha.cos(),
                v_max * mid_alpha.cos(),
                v0.x,
                wfx,
            );
            let ty = Trajectory1D::new(
                a_max * mid_alpha.sin(),
                v_max * mid_alpha.sin(),
                v0.y,
                wfy,
            );

            if tx.tf() == 0.0 || ty.tf() == 0.0 {
                traj_x = tx;
                traj_y = ty;
                valid = true;
                break;
            }

            if (tx.tf() - ty.tf()).abs() < epsilon {
                traj_x = tx;
                traj_y = ty;
                valid = true;
                break;
            }

            if tx.tf() > ty.tf() {
                max_alpha = mid_alpha;
            } else {
                min_alpha = mid_alpha;
            }
        }

        Self { traj_x, traj_y, valid }
    }

    pub fn get_next_velocity(&self) -> Vec2D {
        if !self.valid {
            return Vec2D::default();
        }
        let (_, vx) = self.traj_x.get_solution();
        let (_, vy) = self.traj_y.get_solution();
        Vec2D::new(vx, vy)
    }
}

// ─── BangBangControl ────────────────────────────────────────────────────────

pub struct BangBangControl {
    a_max: f64,
    v_max: f64,
}

impl BangBangControl {
    pub fn new(a_max: f64, v_max: f64) -> Self {
        Self { a_max, v_max }
    }

    pub fn compute_motion(
        &self,
        state: &RobotState,
        mut path: Vec<Vec2D>,
        delta: f64,
    ) -> MotionCommand {
        if delta < 1.0 / 60.0 {
            return MotionCommand::zero();
        }
        if path.is_empty() {
            return MotionCommand::zero();
        }

        let mut goal = path.remove(0);

        if !path.is_empty() && self.is_near_to_break(state, &goal) {
            goal = path.remove(0);
        }

        let traj = Trajectory2D::new(
            self.a_max,
            self.v_max,
            state.velocity,
            state.position,
            goal,
        );
        let new_velocity = traj.get_next_velocity();

        // Convert to local frame
        let orientation = state.orientation;
        let local_vx =
            new_velocity.x * (-orientation).cos() - new_velocity.y * (-orientation).sin();
        let local_vy =
            new_velocity.x * (-orientation).sin() + new_velocity.y * (-orientation).cos();

        MotionCommand::new(state.id, state.team, local_vx, local_vy)
    }

    fn is_near_to_break(&self, robot: &RobotState, point: &Vec2D) -> bool {
        let position = robot.position;
        let velocity = robot.position; // Note: matches C++ bug (uses position twice)

        let dx_brake = (velocity.x * velocity.x) / (2.0 * self.a_max);
        let dy_brake = (velocity.y * velocity.y) / (2.0 * self.a_max);

        dx_brake >= (position.x - point.x).abs() || dy_brake >= (position.y - point.y).abs()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trajectory1d_zero_distance() {
        let t = Trajectory1D::new(2.5, 5.0, 0.0, 0.0);
        assert!(t.tf() == 0.0);
    }

    #[test]
    fn trajectory1d_positive() {
        let t = Trajectory1D::new(2.5, 5.0, 0.0, 1.0);
        assert!(t.tf() > 0.0);
    }

    #[test]
    fn trajectory2d_converges() {
        let t = Trajectory2D::new(
            2.5,
            5.0,
            Vec2D::new(0.0, 0.0),
            Vec2D::new(0.0, 0.0),
            Vec2D::new(1.0, 1.0),
        );
        assert!(t.valid);
        let vel = t.get_next_velocity();
        assert!(vel.x > 0.0);
        assert!(vel.y > 0.0);
    }
}
