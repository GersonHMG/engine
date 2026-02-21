// motion.rs — High-level motion control
// Port of motion/motion.cpp

pub mod controllers;
pub mod pathplanning;

use crate::environment::Environment;
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

    /// Motion with PID velocity control and obstacle avoidance.
    pub fn motion(
        &self,
        robot: &RobotState,
        target: Vec2D,
        world: &World,
        kp_x: f64,
        ki_x: f64,
        kp_y: f64,
        ki_y: f64,
    ) -> MotionCommand {
        let env = Environment::new(world, robot);
        let path = self.planner.get_path(robot.position, target, &env);
        let delta = 1.0 / 60.0;
        let ref_vel = self.bangbang.compute_motion(robot, path, delta);

        let orientation = robot.orientation;
        let _local_velocity = Vec2D::new(
            robot.velocity.x * (-orientation).cos() - robot.velocity.y * (-orientation).sin(),
            robot.velocity.x * (-orientation).sin() + robot.velocity.y * (-orientation).cos(),
        );

        // PID controllers (not actually modifying output in original C++ code either)
        let mut _pid_x = PID::new(kp_x, ki_x, 0.0);
        let mut _pid_y = PID::new(kp_y, ki_y, 0.0);

        MotionCommand::new(robot.id, robot.team, ref_vel.vx, ref_vel.vy)
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
        cmd.angular = angular_velocity;
        cmd
    }

    /// Rotate to a specific angle using PID.
    pub fn face_to_angle(
        &self,
        robot: &RobotState,
        target_angle: f64,
        kp: f64,
        ki: f64,
    ) -> MotionCommand {
        let mut pid = PID::new(kp, ki, 0.0);

        let current = normalize_angle(robot.orientation);
        let target = normalize_angle(target_angle);
        let error = normalize_angle(target - current);

        let delta = 1.0 / 60.0;
        let angular_velocity = pid.compute(error, delta);

        let mut cmd = MotionCommand::with_id_team(robot.id, robot.team);
        cmd.angular = angular_velocity;
        cmd
    }

    /// Motion with orientation control.
    pub fn motion_with_orientation(
        &self,
        robot: &RobotState,
        target: Vec2D,
        target_angle: f64,
        world: &World,
        kp_x: f64,
        ki_x: f64,
        kp_y: f64,
        ki_y: f64,
        kp_angle: f64,
        ki_angle: f64,
    ) -> MotionCommand {
        let env = Environment::new(world, robot);
        let path = self.planner.get_path(robot.position, target, &env);
        let delta = 1.0 / 60.0;
        let ref_vel = self.bangbang.compute_motion(robot, path, delta);

        let orientation = robot.orientation;
        let local_velocity = Vec2D::new(
            robot.velocity.x * (-orientation).cos() - robot.velocity.y * (-orientation).sin(),
            robot.velocity.x * (-orientation).sin() + robot.velocity.y * (-orientation).cos(),
        );

        // PID for Vx
        let mut pid_x = PID::new(kp_x, ki_x, 0.0);
        let error_x = ref_vel.vx - local_velocity.x;
        let control_vx = pid_x.compute(error_x, delta);

        // PID for Vy
        let mut pid_y = PID::new(kp_y, ki_y, 0.0);
        let error_y = ref_vel.vy - local_velocity.y;
        let control_vy = pid_y.compute(error_y, delta);

        // PID for angular
        let mut angle_pid = PID::new(kp_angle, ki_angle, 0.0);
        let current = normalize_angle(robot.orientation);
        let target_a = normalize_angle(target_angle);
        let error_angle = normalize_angle(target_a - current);
        let angular = angle_pid.compute(error_angle, delta);

        let mut cmd = MotionCommand::new(
            robot.id,
            robot.team,
            ref_vel.vx + control_vx,
            ref_vel.vy + control_vy,
        );
        cmd.angular = angular;
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
