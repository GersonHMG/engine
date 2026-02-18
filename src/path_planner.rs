// path_planner.rs — Fast path planner with obstacle avoidance
// Port of motion/path_planner.cpp

use crate::environment::Environment;
use crate::types::Vec2D;

struct Trajectory {
    start: Vec2D,
    goal: Vec2D,
}

fn rotate_vector(v: &Vec2D, angle: f64) -> Vec2D {
    let (sin_a, cos_a) = angle.sin_cos();
    Vec2D::new(cos_a * v.x - sin_a * v.y, sin_a * v.x + cos_a * v.y)
}

pub struct FastPathPlanner {
    max_depth: i32,
}

impl FastPathPlanner {
    pub fn new(max_depth: i32) -> Self {
        Self { max_depth }
    }

    pub fn get_path(&self, from: Vec2D, to: Vec2D, env: &Environment) -> Vec<Vec2D> {
        let raw_path = self.create_path(from, to, env);
        if raw_path.is_empty() {
            return vec![from, to]; // Fallback
        }

        let mut points = Vec::new();
        for seg in &raw_path {
            points.push(seg.start);
        }
        if let Some(last) = raw_path.last() {
            points.push(last.goal);
        }

        self.simplify_path(&points, env)
    }

    fn trajectory_collides(&self, traj: &Trajectory, env: &Environment) -> bool {
        for i in 0..200 {
            let t = i as f64 / 200.0;
            let dir = traj.goal - traj.start;
            let point = traj.start + dir * t;
            if env.collides(&point) {
                return true;
            }
        }
        false
    }

    fn search_subgoal(
        &self,
        traj: &Trajectory,
        env: &Environment,
        robot_diameter: f64,
        direction: f64,
    ) -> Option<Vec2D> {
        let obs_point = traj.goal;
        let mut dir_vec = (obs_point - traj.start).normalized();

        for _ in 0..10 {
            let perp = Vec2D::new(-dir_vec.y, dir_vec.x);
            let offset = perp * (direction * robot_diameter);
            let subgoal = obs_point + offset;

            if !env.collides(&subgoal)
                && subgoal.x >= -4.5
                && subgoal.x <= 4.5
                && subgoal.y >= -3.0
                && subgoal.y <= 3.0
            {
                return Some(subgoal);
            }

            dir_vec = rotate_vector(&dir_vec, std::f64::consts::FRAC_PI_4);
        }

        None
    }

    fn create_path(&self, start: Vec2D, goal: Vec2D, env: &Environment) -> Vec<Trajectory> {
        // Path A (right-hand rule)
        let result_a = self.create_path_direction(start, goal, env, 1.0);

        // Path B (left-hand rule)
        let result_b = self.create_path_direction(start, goal, env, -1.0);

        if result_a.len() <= result_b.len() && !result_a.is_empty() {
            result_a
        } else {
            result_b
        }
    }

    fn create_path_direction(
        &self,
        start: Vec2D,
        goal: Vec2D,
        env: &Environment,
        direction: f64,
    ) -> Vec<Trajectory> {
        let mut result = Vec::new();
        let mut stack: Vec<(Trajectory, i32)> = vec![(Trajectory { start, goal }, 0)];

        while let Some((traj, depth)) = stack.pop() {
            if self.trajectory_collides(&traj, env) && depth < self.max_depth {
                if let Some(sub) = self.search_subgoal(&traj, env, 0.36, direction) {
                    stack.push((
                        Trajectory {
                            start: sub,
                            goal: traj.goal,
                        },
                        depth + 1,
                    ));
                    stack.push((
                        Trajectory {
                            start: traj.start,
                            goal: sub,
                        },
                        depth + 1,
                    ));
                } else {
                    return vec![];
                }
            } else {
                result.push(traj);
            }
        }

        result
    }

    fn simplify_path(&self, path: &[Vec2D], env: &Environment) -> Vec<Vec2D> {
        if path.len() <= 2 {
            return path.to_vec();
        }

        let mut result = vec![path[0]];
        let mut i = 0;

        while i < path.len() - 1 {
            let mut j = path.len() - 1;
            while j > i + 1 {
                let traj = Trajectory {
                    start: path[i],
                    goal: path[j],
                };
                if !self.trajectory_collides(&traj, env) {
                    break;
                }
                j -= 1;
            }
            result.push(path[j]);
            i = j;
        }

        result
    }
}

impl Default for FastPathPlanner {
    fn default() -> Self {
        Self::new(10)
    }
}
