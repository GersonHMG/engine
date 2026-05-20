// environment.rs — Collision detection environment for path planning
// Port of motion/environment.cpp

use crate::types::{RobotState, Vec2D};
use crate::world::World;

/// Environment captures obstacle positions for collision checks.
pub struct Environment {
    robots: Vec<Vec2D>,
    ball: Vec2D,
    field_half_length: f64,
    field_half_width: f64,
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

        let ball = world.get_ball_state().position;
        let field_half_length = world.field_half_length();
        let field_half_width = world.field_half_width();

        Self {
            robots,
            ball,
            field_half_length,
            field_half_width,
        }
    }

    pub fn within_field(&self, point: &Vec2D) -> bool {
        point.x >= -self.field_half_length
            && point.x <= self.field_half_length
            && point.y >= -self.field_half_width
            && point.y <= self.field_half_width
    }

    /// Check if a point collides with any obstacle.
    pub fn collides(&self, point: &Vec2D) -> bool {
        const ROBOT_COLLISION_RADIUS: f64 = 0.2;
        const BALL_COLLISION_RADIUS: f64 = 0.12;
        const GOAL_BOX_DEPTH: f64 = 1.0;
        const GOAL_BOX_HALF_WIDTH: f64 = 1.0;

        // Field bounds
        if !self.within_field(point) {
            return true;
        }

        // Yellow goalie box (x: 3.5 to 4.5, y: -1 to 1)
        if point.x >= self.field_half_length - GOAL_BOX_DEPTH
            && point.x <= self.field_half_length
            && point.y >= -GOAL_BOX_HALF_WIDTH
            && point.y <= GOAL_BOX_HALF_WIDTH
        {
            return true;
        }

        // Blue goalie box (x: -4.5 to -3.5, y: -1 to 1)
        if point.x >= -self.field_half_length
            && point.x <= -self.field_half_length + GOAL_BOX_DEPTH
            && point.y >= -GOAL_BOX_HALF_WIDTH
            && point.y <= GOAL_BOX_HALF_WIDTH
        {
            return true;
        }

        // Robot collision
        for robot_pos in &self.robots {
            if (*point - *robot_pos).length() <= ROBOT_COLLISION_RADIUS {
                return true;
            }
        }

        // Ball collision
        if (*point - self.ball).length() <= BALL_COLLISION_RADIUS {
            return true;
        }

        false
    }
}