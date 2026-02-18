// environment.rs — Collision detection environment for path planning
// Port of motion/environment.cpp

use crate::types::{RobotState, Vec2D};
use crate::world::World;

/// Environment captures obstacle positions for collision checks.
pub struct Environment {
    robots: Vec<Vec2D>,
    ball_position: Vec2D,
}

impl Environment {
    /// Build an environment from the world state, excluding the `self_robot`.
    pub fn new(world: &World, self_robot: &RobotState) -> Self {
        let self_id = self_robot.id;
        let mut robots = Vec::new();

        for id in 0..12 {
            if id == self_id {
                continue;
            }

            let r_blue = world.get_robot_state(id, 0);
            if r_blue.active {
                robots.push(r_blue.position);
            }

            let r_yellow = world.get_robot_state(id, 1);
            if r_yellow.active {
                robots.push(r_yellow.position);
            }
        }

        let ball_position = world.get_ball_state().position;

        Self {
            robots,
            ball_position,
        }
    }

    /// Check if a point collides with any obstacle.
    pub fn collides(&self, point: &Vec2D) -> bool {
        // Field bounds
        if point.x < -4.5 || point.x > 4.5 || point.y < -3.0 || point.y > 3.0 {
            return true;
        }

        // Yellow goalie box (x: 3.5 to 4.5, y: -1 to 1)
        if point.x >= 3.5 && point.x <= 4.5 && point.y >= -1.0 && point.y <= 1.0 {
            return true;
        }

        // Blue goalie box (x: -4.5 to -3.5, y: -1 to 1)
        if point.x >= -4.5 && point.x <= -3.5 && point.y >= -1.0 && point.y <= 1.0 {
            return true;
        }

        // Robot collision
        for robot_pos in &self.robots {
            if (*point - *robot_pos).length() <= 0.2 {
                return true;
            }
        }

        // Ball collision
        if (self.ball_position - *point).length() <= 0.1 {
            return true;
        }

        false
    }

    pub fn get_robots(&self) -> &[Vec2D] {
        &self.robots
    }

    pub fn get_ball_position(&self) -> &Vec2D {
        &self.ball_position
    }
}
