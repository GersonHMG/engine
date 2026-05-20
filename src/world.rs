// world.rs — World state management
// Port of world/world.cpp

use crate::types::{BallState, RobotState, Vec2D};
use std::collections::HashMap;
use std::time::{Duration, Instant};

// 60 * 4 frames at ~60 FPS.
const ROBOT_STALE_MAX_AGE: Duration = Duration::from_secs(3);

pub struct World {
    pub blue_robots: HashMap<i32, RobotState>,
    pub yellow_robots: HashMap<i32, RobotState>,
    pub ball: BallState,
    pub field_length_m: f64,
    pub field_width_m: f64,
}

impl World {
    pub fn new(n_blue: i32, n_yellow: i32, field_length_m: f64, field_width_m: f64) -> Self {
        let mut blue = HashMap::new();
        for id in 0..n_blue {
            blue.insert(id, RobotState::new(id, 0));
        }
        let mut yellow = HashMap::new();
        for id in 0..n_yellow {
            yellow.insert(id, RobotState::new(id, 1));
        }
        Self {
            blue_robots: blue,
            yellow_robots: yellow,
            ball: BallState::default(),
            field_length_m,
            field_width_m,
        }
    }

    pub fn field_half_length(&self) -> f64 {
        self.field_length_m / 2.0
    }

    pub fn field_half_width(&self) -> f64 {
        self.field_width_m / 2.0
    }

    pub fn get_robot_state(&self, id: i32, team: i32) -> RobotState {
        match team {
            0 => self.blue_robots.get(&id).cloned().unwrap_or_default(),
            1 => self.yellow_robots.get(&id).cloned().unwrap_or_default(),
            _ => RobotState::default(),
        }
    }

    pub fn get_ball_state(&self) -> BallState {
        self.ball.clone()
    }

    pub fn get_blue_team_state(&self) -> Vec<RobotState> {
        self.blue_robots.values().cloned().collect()
    }

    pub fn get_yellow_team_state(&self) -> Vec<RobotState> {
        self.yellow_robots.values().cloned().collect()
    }

    pub fn update_robot(
        &mut self,
        id: i32,
        team: i32,
        position: Vec2D,
        orientation: f32,
        velocity: Vec2D,
        omega: f32,
    ) {
        let robots = match team {
            0 => &mut self.blue_robots,
            _ => &mut self.yellow_robots,
        };

        let robot = robots.entry(id).or_insert_with(|| RobotState::new(id, team));
        robot.position = position;
        robot.orientation = orientation as f64;
        robot.velocity = velocity;
        robot.angular_velocity = omega as f64;
        robot.active = true;
        robot.last_update = Instant::now();
    }

    pub fn update_ball(&mut self, velocity: Vec2D, position: Vec2D) {
        self.ball.position = position;
        self.ball.velocity = velocity;
    }

    pub fn prune_stale_robots(&mut self) {
        let now = Instant::now();
        self.blue_robots.retain(|_, robot| {
            robot.active = now.duration_since(robot.last_update) <= ROBOT_STALE_MAX_AGE;
            robot.active
        });
        self.yellow_robots.retain(|_, robot| {
            robot.active = now.duration_since(robot.last_update) <= ROBOT_STALE_MAX_AGE;
            robot.active
        });
    }

    pub fn set_commanded_velocity(&mut self, id: i32, team: i32, cmd_v: Vec2D, cmd_angular: f64) {
        let robots = match team {
            0 => &mut self.blue_robots,
            _ => &mut self.yellow_robots,
        };
        if let Some(robot) = robots.get_mut(&id) {
            robot.commanded_velocity = cmd_v;
            robot.commanded_angular = cmd_angular;
        }
    }

}
