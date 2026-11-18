use serde::{Deserialize, Serialize};

/// Longest vision or command delay, s.
pub const MAX_DELAY: f64 = 1.0;

/// All physical parameters. Every field has a default, so a config file only
/// needs the values it changes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SimConfig {
    pub field: FieldConfig,
    pub robot: RobotConfig,
    pub ball: BallConfig,
    /// What the AI sees: noise and latency of the vision frames.
    pub vision: VisionConfig,
    /// Latency of the robot commands.
    pub radio: RadioConfig,
    /// Longest physics substep, s. `Simulator::step` splits its `dt` into
    /// substeps no longer than this.
    pub max_substep: f64,
}

impl SimConfig {
    /// Checks that every parameter is usable by the physics, naming the
    /// first one that is not.
    pub fn validate(&self) -> Result<(), String> {
        let positive = |v: f64| v.is_finite() && v > 0.0;
        let non_negative = |v: f64| v.is_finite() && v >= 0.0;
        let fraction = |v: f64| v.is_finite() && (0.0..=1.0).contains(&v);
        let delay = |v: f64| v.is_finite() && (0.0..=MAX_DELAY).contains(&v);
        let (field, robot, ball) = (&self.field, &self.robot, &self.ball);

        let checks = [
            ("max_substep", self.max_substep, positive(self.max_substep), "> 0"),
            ("field.length", field.length, positive(field.length), "> 0"),
            ("field.width", field.width, positive(field.width), "> 0"),
            ("field.boundary_width", field.boundary_width, non_negative(field.boundary_width), ">= 0"),
            ("robot.radius", robot.radius, positive(robot.radius), "> 0"),
            (
                "robot.front_distance",
                robot.front_distance,
                non_negative(robot.front_distance) && robot.front_distance < robot.radius,
                ">= 0 and < robot.radius",
            ),
            ("robot.max_speed", robot.max_speed, non_negative(robot.max_speed), ">= 0"),
            ("robot.max_accel", robot.max_accel, non_negative(robot.max_accel), ">= 0"),
            ("robot.max_omega", robot.max_omega, non_negative(robot.max_omega), ">= 0"),
            ("robot.max_alpha", robot.max_alpha, non_negative(robot.max_alpha), ">= 0"),
            ("robot.grip_accel", robot.grip_accel, non_negative(robot.grip_accel), ">= 0"),
            ("robot.slide_grip", robot.slide_grip, fraction(robot.slide_grip), "in [0, 1]"),
            ("robot.slip_spin", robot.slip_spin, non_negative(robot.slip_spin), ">= 0"),
            ("robot.wheel_speed", robot.wheel_speed, non_negative(robot.wheel_speed), ">= 0"),
            (
                "robot.front_wheel_angle",
                robot.front_wheel_angle,
                robot.front_wheel_angle.is_finite() && robot.front_wheel_angle > 0.0 && robot.front_wheel_angle < robot.back_wheel_angle,
                "> 0 and < robot.back_wheel_angle",
            ),
            (
                "robot.back_wheel_angle",
                robot.back_wheel_angle,
                robot.back_wheel_angle.is_finite() && robot.back_wheel_angle < std::f64::consts::PI,
                "< π",
            ),
            (
                "robot.wheel_distance",
                robot.wheel_distance,
                positive(robot.wheel_distance) && robot.wheel_distance <= robot.radius,
                "> 0 and <= robot.radius",
            ),
            ("robot.max_kick_speed", robot.max_kick_speed, non_negative(robot.max_kick_speed), ">= 0"),
            ("robot.kick_x_speed", robot.kick_x_speed, non_negative(robot.kick_x_speed), ">= 0"),
            ("robot.kick_reach", robot.kick_reach, non_negative(robot.kick_reach), ">= 0"),
            ("robot.body_restitution", robot.body_restitution, fraction(robot.body_restitution), "in [0, 1]"),
            ("robot.front_restitution", robot.front_restitution, fraction(robot.front_restitution), "in [0, 1]"),
            ("ball.radius", ball.radius, positive(ball.radius), "> 0"),
            ("ball.slide_decel", ball.slide_decel, non_negative(ball.slide_decel), ">= 0"),
            ("ball.roll_decel", ball.roll_decel, non_negative(ball.roll_decel), ">= 0"),
            ("ball.wall_restitution", ball.wall_restitution, fraction(ball.wall_restitution), "in [0, 1]"),
            ("vision.position_noise", self.vision.position_noise, non_negative(self.vision.position_noise), ">= 0"),
            ("vision.angle_noise", self.vision.angle_noise, non_negative(self.vision.angle_noise), ">= 0"),
            ("vision.delay", self.vision.delay, delay(self.vision.delay), "in [0, 1]"),
            ("radio.delay", self.radio.delay, delay(self.radio.delay), "in [0, 1]"),
        ];

        match checks.iter().find(|(_, _, ok, _)| !ok) {
            Some((name, value, _, rule)) => Err(format!("{name} must be {rule}, got {value}")),
            None => Ok(()),
        }
    }
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            field: FieldConfig::default(),
            robot: RobotConfig::default(),
            ball: BallConfig::default(),
            vision: VisionConfig::default(),
            radio: RadioConfig::default(),
            max_substep: 0.001,
        }
    }
}

/// Playing area plus the boundary strip; the walls close the outer edge.
/// Defaults: SSL division B.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FieldConfig {
    /// Playing area length (along x), m.
    pub length: f64,
    /// Playing area width (along y), m.
    pub width: f64,
    /// Strip between the field lines and the walls, m.
    pub boundary_width: f64,
}

impl FieldConfig {
    /// x of the right wall; the left wall is at -wall_x().
    pub fn wall_x(&self) -> f64 {
        self.length / 2.0 + self.boundary_width
    }

    /// y of the top wall; the bottom wall is at -wall_y().
    pub fn wall_y(&self) -> f64 {
        self.width / 2.0 + self.boundary_width
    }
}

impl Default for FieldConfig {
    fn default() -> Self {
        Self {
            length: 9.0,
            width: 6.0,
            boundary_width: 0.3,
        }
    }
}

/// Robot body and actuators. The body is a circle cut flat at the front,
/// where the kicker is.
///
/// The speed and acceleration limits are the hardware's; motion controllers
/// are expected to stay below them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RobotConfig {
    /// m.
    pub radius: f64,
    /// Distance from the center to the flat front face, m.
    pub front_distance: f64,
    /// m/s.
    pub max_speed: f64,
    /// m/s².
    pub max_accel: f64,
    /// Most acceleration the floor can give the body, m/s² (friction).
    /// 0 = perfect grip: the body follows the commanded velocity within
    /// `max_accel` and never slides. Above 0, `max_accel` is what the motors
    /// can do to the wheels; asking the body for more than `grip_accel`
    /// (starting, braking, or turning while driving) makes the wheels slip.
    pub grip_accel: f64,
    /// Grip left while sliding, as a fraction of `grip_accel` (kinetic /
    /// static friction): once a robot slides it accelerates and brakes worse
    /// until its wheels match the floor again.
    pub slide_grip: f64,
    /// Heading drift while sliding, rad/s per m/s of slip (counter-clockwise):
    /// slipping wheels push unevenly and the robot turns.
    pub slip_spin: f64,
    /// Most ground speed a wheel can reach, m/s. 0 = no wheel limit: the
    /// robot reaches `max_speed` in every direction. Above 0, a command that
    /// would spin any wheel faster is scaled down as a whole, so how fast the
    /// robot goes depends on the direction (see the wheel angles).
    pub wheel_speed: f64,
    /// Angle of the two front omni wheels from the forward axis, rad (one on
    /// each side, mirrored). Each wheel rolls at right angles to its axle.
    pub front_wheel_angle: f64,
    /// Angle of the two back omni wheels from the forward axis, rad.
    pub back_wheel_angle: f64,
    /// Distance from the center to the wheels' contact points, m.
    pub wheel_distance: f64,
    /// rad/s.
    pub max_omega: f64,
    /// rad/s².
    pub max_alpha: f64,
    /// m/s, SSL rule limit.
    pub max_kick_speed: f64,
    /// Speed of the flat kick the engine commands for `kickx`, m/s (the kick
    /// itself is still capped by `max_kick_speed`).
    pub kick_x_speed: f64,
    /// How far in front of the face the ball may be and still get kicked, m.
    pub kick_reach: f64,
    /// Ball bounce off the round body.
    pub body_restitution: f64,
    /// Ball bounce off the front face (damped by the dribbler bar).
    pub front_restitution: f64,
}

impl RobotConfig {
    /// Half the width of the flat front face, m.
    pub fn front_half_width(&self) -> f64 {
        (self.radius * self.radius - self.front_distance * self.front_distance).sqrt()
    }

    /// Where the four wheels are, counter-clockwise from the forward axis,
    /// rad: front left, back left, back right, front right.
    pub fn wheel_angles(&self) -> [f64; 4] {
        let (f, b) = (self.front_wheel_angle, self.back_wheel_angle);
        [f, b, -b, -f]
    }

    /// Ground speed of each wheel (in `wheel_angles` order) for a velocity in
    /// the robot's frame, m/s.
    pub fn wheel_speeds(&self, local_velocity: crate::Vec2, omega: f64) -> [f64; 4] {
        self.wheel_angles()
            .map(|phi| -phi.sin() * local_velocity.x + phi.cos() * local_velocity.y + self.wheel_distance * omega)
    }

    /// The command (robot frame) scaled down so no wheel goes past
    /// `wheel_speed`; unchanged when `wheel_speed` is 0.
    pub fn wheel_limited(&self, local_velocity: crate::Vec2, omega: f64) -> (crate::Vec2, f64) {
        if self.wheel_speed <= 0.0 {
            return (local_velocity, omega);
        }
        let fastest = self.wheel_speeds(local_velocity, omega).iter().fold(0.0_f64, |m, s| m.max(s.abs()));
        if fastest <= self.wheel_speed {
            return (local_velocity, omega);
        }
        let k = self.wheel_speed / fastest;
        (local_velocity * k, omega * k)
    }

    /// Top speed driving (without turning) in `direction`, rad from the
    /// forward axis in the robot's frame, m/s.
    pub fn top_speed_towards(&self, direction: f64) -> f64 {
        let unit = crate::Vec2::new(direction.cos(), direction.sin());
        let (v, _) = self.wheel_limited(unit * self.max_speed, 0.0);
        v.length()
    }
}

impl Default for RobotConfig {
    fn default() -> Self {
        Self {
            radius: 0.09,
            front_distance: 0.073,
            max_speed: 3.5,
            max_accel: 6.0,
            grip_accel: 0.0,
            slide_grip: 1.0,
            slip_spin: 0.0,
            wheel_speed: 0.0,
            front_wheel_angle: std::f64::consts::PI / 3.0,
            back_wheel_angle: 3.0 * std::f64::consts::PI / 4.0,
            wheel_distance: 0.08,
            max_omega: 10.0,
            max_alpha: 40.0,
            max_kick_speed: 6.5,
            kick_x_speed: 3.0,
            kick_reach: 0.01,
            body_restitution: 0.5,
            front_restitution: 0.2,
        }
    }
}

/// Ball. After an impulse the ball slides (high friction) until it has
/// slowed to 5/7 of its speed, then rolls (low friction).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BallConfig {
    /// m.
    pub radius: f64,
    /// Deceleration while sliding, m/s².
    pub slide_decel: f64,
    /// Deceleration while rolling, m/s².
    pub roll_decel: f64,
    /// Bounce off the walls.
    pub wall_restitution: f64,
}

impl Default for BallConfig {
    fn default() -> Self {
        Self {
            radius: 0.0215,
            slide_decel: 3.0,
            roll_decel: 0.35,
            wall_restitution: 0.6,
        }
    }
}

/// Vision as the AI receives it. The physics does not use these: whoever
/// turns the simulation into vision frames (the engine) applies them.
/// All zero = perfect, instant vision.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct VisionConfig {
    /// Standard deviation of the gaussian noise on robot and ball positions, m.
    pub position_noise: f64,
    /// Standard deviation of the gaussian noise on robot headings, rad.
    pub angle_noise: f64,
    /// Age of a frame when the AI receives it, s.
    pub delay: f64,
}

/// Robot commands as the robots receive them. Applied by whoever feeds the
/// commands to the simulator (the engine). Zero = instant.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RadioConfig {
    /// Time from sending a command to the robot acting on it, s.
    pub delay: f64,
}
