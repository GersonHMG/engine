use serde::{Deserialize, Serialize};

/// All physical parameters. Every field has a default, so a config file only
/// needs the values it changes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SimConfig {
    pub field: FieldConfig,
    pub robot: RobotConfig,
    pub ball: BallConfig,
    /// Longest physics substep, s. `Simulator::step` splits its `dt` into
    /// substeps no longer than this.
    pub max_substep: f64,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            field: FieldConfig::default(),
            robot: RobotConfig::default(),
            ball: BallConfig::default(),
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
    /// rad/s.
    pub max_omega: f64,
    /// rad/s².
    pub max_alpha: f64,
    /// m/s, SSL rule limit.
    pub max_kick_speed: f64,
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
}

impl Default for RobotConfig {
    fn default() -> Self {
        Self {
            radius: 0.09,
            front_distance: 0.073,
            max_speed: 3.5,
            max_accel: 6.0,
            max_omega: 10.0,
            max_alpha: 40.0,
            max_kick_speed: 6.5,
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
