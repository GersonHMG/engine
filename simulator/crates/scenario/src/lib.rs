//! Scenario files (TOML): initial state, a timed command script and
//! optional physics overrides.
//!
//! ```toml
//! name = "drive_and_kick"
//! duration = 5.0
//!
//! [robot]
//! x = -1.0
//!
//! [[commands]]        # in effect from `t` until the next entry
//! t = 0.0
//! vt = 1.0
//! kick_speed = 3.0
//! ```

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use ssl_sim::{Ball, Robot, RobotCommand, RobotId, SimConfig, SimState, Team, Vec2};
use std::path::Path;

/// The robot a scenario places and drives.
pub const SCENARIO_ROBOT: RobotId = RobotId::new(Team::Blue, 0);

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    #[serde(default)]
    pub name: String,
    /// Simulated time, s.
    pub duration: f64,
    /// Control period: how often the command script is read, s.
    #[serde(default = "default_tick")]
    pub tick: f64,
    #[serde(default)]
    pub robot: RobotInit,
    #[serde(default)]
    pub ball: BallInit,
    #[serde(default)]
    pub commands: Vec<TimedCommand>,
    #[serde(default)]
    pub config: SimConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RobotInit {
    pub x: f64,
    pub y: f64,
    pub theta: f64,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BallInit {
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
}

#[derive(Debug, Deserialize)]
pub struct TimedCommand {
    /// Start time, s.
    pub t: f64,
    #[serde(flatten)]
    pub command: RobotCommand,
}

fn default_tick() -> f64 {
    1.0 / 60.0
}

fn require_positive(name: &str, value: f64) -> Result<()> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        bail!("{name} must be a finite number > 0, got {value}")
    }
}

impl Scenario {
    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("cannot read scenario {}", path.display()))?;
        let mut scenario: Scenario = toml::from_str(&text)
            .with_context(|| format!("invalid scenario {}", path.display()))?;
        if scenario.name.is_empty() {
            scenario.name = path.file_stem().unwrap_or_default().to_string_lossy().into_owned();
        }
        scenario.validate()?;
        Ok(scenario)
    }

    fn validate(&self) -> Result<()> {
        require_positive("duration", self.duration)?;
        require_positive("tick", self.tick)?;
        require_positive("config.max_substep", self.config.max_substep)?;
        if self.commands.windows(2).any(|pair| pair[1].t < pair[0].t) {
            bail!("commands must be sorted by t");
        }
        Ok(())
    }

    pub fn initial_state(&self) -> SimState {
        let robot = Robot::new(SCENARIO_ROBOT, Vec2::new(self.robot.x, self.robot.y), self.robot.theta);
        let ball = Ball::new(
            Vec2::new(self.ball.x, self.ball.y),
            Vec2::new(self.ball.vx, self.ball.vy),
        );
        SimState::new(vec![robot], ball)
    }

    /// Index of the command in effect at time `t`, if any. Tolerates the
    /// rounding of `t` accumulated over ticks.
    fn command_index_at(&self, t: f64) -> Option<usize> {
        const TIME_EPSILON: f64 = 1e-9;
        self.commands.iter().rposition(|c| c.t <= t + TIME_EPSILON)
    }
}

/// Plays a scenario's command script. It hands out a command only when a
/// new one takes effect, so each entry's kick fires at most once.
#[derive(Debug, Default)]
pub struct ScriptCursor {
    current: Option<usize>,
}

impl ScriptCursor {
    /// The command to apply at time `t`, or `None` if the one in effect
    /// has not changed.
    pub fn advance(&mut self, scenario: &Scenario, t: f64) -> Option<RobotCommand> {
        let index = scenario.command_index_at(t);
        if index == self.current {
            return None;
        }
        self.current = index;
        Some(index.map_or_else(RobotCommand::default, |i| scenario.commands[i].command))
    }
}
