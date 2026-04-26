// motion.rs — High-level motion control
// Port of motion/motion.cpp

pub mod controllers;
pub mod pathplanning;

use crate::motion::pathplanning::environment::Environment;
use crate::motion::pathplanning::FastPathPlanner;
use crate::motion::controllers::pid::pid::PID;
use crate::motion::controllers::bangbang::BangBangControl;
use crate::types::{MotionCommand, RobotState, Vec2D};
use crate::world::World;

/// Motion controller providing high-level movement commands.
pub struct Motion {
    bangbang: BangBangControl,
    planner: FastPathPlanner,
}

impl Default for Motion {
    fn default() -> Self {
        Self {
            bangbang: BangBangControl::new(2.5, 5.0),
            planner: FastPathPlanner::default(),
        }
    }
}

impl Motion {
    pub fn new() -> Self {
        Self::default()
    }

    /// Move without obstacle avoidance (direct path).
    pub fn move_direct(&self, robot: &RobotState, target: Vec2D) -> MotionCommand {
        let path = vec![robot.position, target];
        let delta = 1.0 / 60.0;
        self.bangbang.compute_motion(robot, path, delta)
    }

    /// Move with obstacle avoidance via path planner.
    pub fn move_to(&self, robot: &RobotState, target: Vec2D, world: &World) -> MotionCommand {
        let env = Environment::new(world, robot);
        let path = self.planner.get_path(robot.position, target, &env);
        let delta = 1.0 / 60.0;
        self.bangbang.compute_motion(robot, path, delta)
    }
    
    /// Rotate to face a target point using PID.
    pub fn face_to(
        &self,
        robot: &RobotState,
        target: Vec2D,
        kp: f64,
        ki: f64,
        kd: f64,
    ) -> MotionCommand {
        let mut pid = PID::new(kp, ki, kd);

        let current_angle = normalize_angle(robot.orientation);
        let target_angle = normalize_angle(
            (target.y - robot.position.y).atan2(target.x - robot.position.x),
        );

        let error = normalize_angle(target_angle - current_angle);
        let delta = 1.0 / 60.0;
        let angular_velocity = pid.compute(error, delta);

        let mut cmd = MotionCommand::with_id_team(robot.id, robot.team);
        cmd.angular = Some(angular_velocity);
        cmd
    }

}

pub fn normalize_angle(mut angle: f64) -> f64 {
    while angle > std::f64::consts::PI {
        angle -= 2.0 * std::f64::consts::PI;
    }
    while angle < -std::f64::consts::PI {
        angle += 2.0 * std::f64::consts::PI;
    }
    angle
}
