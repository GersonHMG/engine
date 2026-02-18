// world.rs — World state management
// Port of world/world.cpp

use crate::types::{BallState, RobotState, Vec2D};
use serde_json::json;
use std::collections::HashMap;
use std::time::Instant;

pub struct World {
    pub blue_robots: HashMap<i32, RobotState>,
    pub yellow_robots: HashMap<i32, RobotState>,
    pub ball: BallState,
}

impl World {
    pub fn new(n_blue: i32, n_yellow: i32) -> Self {
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
        }
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

        if let Some(robot) = robots.get_mut(&id) {
            robot.position = position;
            robot.orientation = orientation as f64;
            robot.velocity = velocity;
            robot.angular_velocity = omega as f64;
            let elapsed = robot.last_update.elapsed();
            robot.active = elapsed.as_millis() <= 2000;
            robot.last_update = Instant::now();
        }
    }

    pub fn update_ball(&mut self, velocity: Vec2D, position: Vec2D) {
        self.ball.position = position;
        self.ball.velocity = velocity;
    }

    pub fn to_json(&self) -> serde_json::Value {
        let mut robots_arr = Vec::new();

        for robot in self.blue_robots.values() {
            if !robot.active {
                continue;
            }
            robots_arr.push(json!({
                "id": robot.id,
                "team": "blue",
                "position": { "x": robot.position.x, "y": robot.position.y },
                "velocity": { "x": robot.velocity.x, "y": robot.velocity.y },
                "orientation": robot.orientation,
            }));
        }

        for robot in self.yellow_robots.values() {
            if !robot.active {
                continue;
            }
            robots_arr.push(json!({
                "id": robot.id,
                "team": "yellow",
                "position": { "x": robot.position.x, "y": robot.position.y },
                "velocity": { "x": robot.velocity.x, "y": robot.velocity.y },
                "orientation": robot.orientation,
            }));
        }

        json!({
            "robots": robots_arr,
            "ball": {
                "position": { "x": self.ball.position.x, "y": self.ball.position.y },
                "velocity": { "x": self.ball.velocity.x, "y": self.ball.velocity.y },
            },
        })
    }
}
