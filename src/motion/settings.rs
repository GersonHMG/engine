// motion/settings.rs — Bang-bang motion limits, for all robots and per robot
//
// `move_to` / `move_direct` plan each robot's motion with a maximum
// acceleration and speed, and every velocity command a robot gets (also
// send_velocity and manual control) changes by at most `ramp_accel` per
// second, so the wheels keep their grip (`motion::traction`). One set
// applies to every robot; a robot can override any value. They are kept in `robot_settings.toml` and changed
// with the `motion ...` console commands,
// so every change shows in the console:
//
//   motion                                  every value
//   motion all <max_accel|max_speed|ramp_accel> <v>    the value for every robot
//   motion <blue|yellow> <id> <param> <v>   one robot's own value
//   motion <blue|yellow> <id> <param> default   back to the all-robots value
//   motion <blue|yellow> <id> clear         drop all of that robot's own values
//   motion save                             write robot_settings.toml

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::{LazyLock, RwLock};
use tracing::{info, warn};

const SETTINGS_FILE: &str = "robot_settings.toml";

const HEADER: &str = "\
# Robot settings, read when the engine starts. Edit them in the Robot
# settings panel or with the `motion ...` console commands (`motion save`
# writes this file).
#
# [bangbang]: limits move_to/move_direct plan with (max_accel, max_speed)
# and how fast any velocity command may change (ramp_accel: what the robot
# can do without slipping, 0 = no ramp), for every robot.
# [[bangbang.robot]]: one robot's own values; a value left out uses the
# all-robots one.

";

/// The limits one robot's motion is planned with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BangBang {
    /// m/s²
    pub max_accel: f64,
    /// m/s
    pub max_speed: f64,
    /// m/s²: most the velocity command may change per second, whatever sent
    /// it (see `motion::traction`): what the robot can do without its wheels
    /// slipping. 0 = commands go out as they are.
    pub ramp_accel: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TeamColor {
    Blue,
    Yellow,
}

impl TeamColor {
    pub fn from_id(team: i32) -> Option<Self> {
        match team {
            0 => Some(Self::Blue),
            1 => Some(Self::Yellow),
            _ => None,
        }
    }

    pub fn word(self) -> &'static str {
        match self {
            Self::Blue => "blue",
            Self::Yellow => "yellow",
        }
    }

    fn parse(word: &str) -> Option<Self> {
        match word {
            "blue" | "b" | "0" => Some(Self::Blue),
            "yellow" | "y" | "1" => Some(Self::Yellow),
            _ => None,
        }
    }
}

/// One robot's own values; `None` uses the all-robots value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RobotBangBang {
    pub team: TeamColor,
    pub id: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_accel: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_speed: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ramp_accel: Option<f64>,
}

impl RobotBangBang {
    pub fn get(&self, param: Param) -> Option<f64> {
        match param {
            Param::MaxAccel => self.max_accel,
            Param::MaxSpeed => self.max_speed,
            Param::RampAccel => self.ramp_accel,
        }
    }

    fn slot(&mut self, param: Param) -> &mut Option<f64> {
        match param {
            Param::MaxAccel => &mut self.max_accel,
            Param::MaxSpeed => &mut self.max_speed,
            Param::RampAccel => &mut self.ramp_accel,
        }
    }

    fn is_empty(&self) -> bool {
        Param::ALL.into_iter().all(|p| self.get(p).is_none())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BangBangSettings {
    pub max_accel: f64,
    pub max_speed: f64,
    pub ramp_accel: f64,
    #[serde(rename = "robot", skip_serializing_if = "Vec::is_empty")]
    pub robots: Vec<RobotBangBang>,
}

impl Default for BangBangSettings {
    fn default() -> Self {
        // Commands ramp at the simulator robot's 2 m/s² (simulator.toml); the
        // plan keeps 10% below it, room to correct for vision and radio delay.
        // Measured with lua/tests/motion_control (README there).
        Self { max_accel: 1.8, max_speed: 5.0, ramp_accel: 2.0, robots: Vec::new() }
    }
}

impl BangBangSettings {
    pub fn get(&self, param: Param) -> f64 {
        match param {
            Param::MaxAccel => self.max_accel,
            Param::MaxSpeed => self.max_speed,
            Param::RampAccel => self.ramp_accel,
        }
    }

    fn slot(&mut self, param: Param) -> &mut f64 {
        match param {
            Param::MaxAccel => &mut self.max_accel,
            Param::MaxSpeed => &mut self.max_speed,
            Param::RampAccel => &mut self.ramp_accel,
        }
    }

    /// The robot's own values, if it has any.
    pub fn robot(&self, team: TeamColor, id: i32) -> Option<&RobotBangBang> {
        self.robots.iter().find(|r| r.team == team && r.id == id)
    }

    /// The limits robot (id, team) runs with.
    pub fn for_robot(&self, id: i32, team: i32) -> BangBang {
        let own = TeamColor::from_id(team).and_then(|team| self.robot(team, id));
        BangBang {
            max_accel: own.and_then(|r| r.max_accel).unwrap_or(self.max_accel),
            max_speed: own.and_then(|r| r.max_speed).unwrap_or(self.max_speed),
            ramp_accel: own.and_then(|r| r.ramp_accel).unwrap_or(self.ramp_accel),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RobotSettings {
    pub bangbang: BangBangSettings,
}

impl RobotSettings {
    /// The planner limits must be numbers > 0 (it divides by them);
    /// ramp_accel may also be 0 (no ramp).
    pub fn validate(&self) -> Result<(), String> {
        let check = |label: String, param: Param, v: f64| {
            if param.valid(v) { Ok(()) } else { Err(format!("{label} must be a number {}", param.rule())) }
        };
        let b = &self.bangbang;
        for param in Param::ALL {
            check(param.word().into(), param, b.get(param))?;
        }
        for r in &b.robots {
            for param in Param::ALL {
                if let Some(v) = r.get(param) {
                    check(format!("{} {} {}", r.team.word(), r.id, param.word()), param, v)?;
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Param {
    MaxAccel,
    MaxSpeed,
    RampAccel,
}

impl Param {
    pub const ALL: [Param; 3] = [Param::MaxAccel, Param::MaxSpeed, Param::RampAccel];

    pub fn word(self) -> &'static str {
        match self {
            Param::MaxAccel => "max_accel",
            Param::MaxSpeed => "max_speed",
            Param::RampAccel => "ramp_accel",
        }
    }

    pub fn valid(self, v: f64) -> bool {
        match self {
            Param::RampAccel => v.is_finite() && v >= 0.0,
            _ => v.is_finite() && v > 0.0,
        }
    }

    pub fn rule(self) -> &'static str {
        match self {
            Param::RampAccel => ">= 0 (0 = no ramp)",
            _ => "> 0",
        }
    }

    fn parse(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.word() == word)
    }
}

// ─── The engine's settings ──────────────────────────────────────────────────

static SETTINGS: LazyLock<RwLock<RobotSettings>> = LazyLock::new(|| RwLock::new(load()));

/// The settings now.
pub fn current() -> RobotSettings {
    SETTINGS.read().unwrap_or_else(|e| e.into_inner()).clone()
}

/// The bang-bang limits robot (id, team) runs with.
pub fn bangbang_for(id: i32, team: i32) -> BangBang {
    SETTINGS.read().unwrap_or_else(|e| e.into_inner()).bangbang.for_robot(id, team)
}

/// Applies `edit` if the result is valid.
fn update(edit: impl FnOnce(&mut RobotSettings) -> Result<(), String>) -> Result<(), String> {
    let mut settings = SETTINGS.write().unwrap_or_else(|e| e.into_inner());
    let mut changed = settings.clone();
    edit(&mut changed)?;
    changed.validate()?;
    *settings = changed;
    Ok(())
}

/// `robot_settings.toml` in the working directory or, failing that, next to
/// the executable. A new file goes in the working directory.
fn settings_path() -> PathBuf {
    let in_working_dir = PathBuf::from(SETTINGS_FILE);
    if in_working_dir.exists() {
        return in_working_dir;
    }
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join(SETTINGS_FILE)))
        .filter(|next_to_exe| next_to_exe.exists())
        .unwrap_or(in_working_dir)
}

fn load() -> RobotSettings {
    let path = settings_path();
    let Ok(text) = fs::read_to_string(&path) else {
        return RobotSettings::default();
    };
    match parse(&text) {
        Ok(settings) => {
            info!("Robot settings loaded from {}", path.display());
            settings
        }
        Err(e) => {
            warn!("Invalid {}: {e}. Using default robot settings.", path.display());
            RobotSettings::default()
        }
    }
}

fn parse(text: &str) -> Result<RobotSettings, String> {
    let settings: RobotSettings = toml::from_str(text).map_err(|e| e.to_string())?;
    settings.validate()?;
    Ok(settings)
}

fn save() -> Result<PathBuf, String> {
    let path = settings_path();
    let text = toml::to_string(&current()).map_err(|e| e.to_string())?;
    fs::write(&path, format!("{HEADER}{text}")).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    Ok(path)
}

// ─── Console ────────────────────────────────────────────────────────────────

pub const HELP: &[&str] = &[
    "motion                                  bang-bang limits (all robots and each robot's own)",
    "motion all <max_accel|max_speed|ramp_accel> <v>  set the limit for every robot (m/s², m/s, m/s²)",
    "motion <blue|yellow> <id> <param> <v>   set one robot's own limit (`default` = use the all-robots one)",
    "motion <blue|yellow> <id> clear         one robot back to the all-robots limits",
    "motion save                             keep these limits in robot_settings.toml",
];

/// Runs `motion <args>`; returns the lines to print.
pub fn console(args: &[&str]) -> Result<Vec<String>, String> {
    match args {
        [] => Ok(describe(&current())),
        ["help"] => Ok(HELP.iter().map(|s| s.to_string()).collect()),
        ["save"] => save().map(|path| vec![format!("saved to {}", path.display())]),
        ["all", param, value] => {
            let param = parse_param(param)?;
            let value = parse_value(param, value)?;
            update(|s| {
                *s.bangbang.slot(param) = value;
                Ok(())
            })?;
            Ok(vec![format!("all robots {} = {value}", param.word())])
        }
        [team, id, "clear"] => {
            let (team, id) = parse_robot(team, id)?;
            update(|s| {
                s.bangbang.robots.retain(|r| !(r.team == team && r.id == id));
                Ok(())
            })?;
            Ok(vec![format!("{} {id} uses the all-robots limits", team.word())])
        }
        [team, id, param, value] => {
            let (team, id) = parse_robot(team, id)?;
            let param = parse_param(param)?;
            let value = if *value == "default" { None } else { Some(parse_value(param, value)?) };
            update(|s| {
                let robots = &mut s.bangbang.robots;
                let index = match robots.iter().position(|r| r.team == team && r.id == id) {
                    Some(i) => i,
                    None => {
                        robots.push(RobotBangBang { team, id, max_accel: None, max_speed: None, ramp_accel: None });
                        robots.sort_by_key(|r| (r.team, r.id));
                        robots.iter().position(|r| r.team == team && r.id == id).unwrap()
                    }
                };
                *robots[index].slot(param) = value;
                if robots[index].is_empty() {
                    robots.remove(index);
                }
                Ok(())
            })?;
            Ok(vec![match value {
                Some(v) => format!("{} {id} {} = {v}", team.word(), param.word()),
                None => format!("{} {id} {} = all-robots value", team.word(), param.word()),
            }])
        }
        _ => Err("usage: motion [all <param> <value> | <blue|yellow> <id> <param> <value|default> | <blue|yellow> <id> clear | save]".into()),
    }
}

fn describe(settings: &RobotSettings) -> Vec<String> {
    let b = &settings.bangbang;
    let mut lines = vec![
        "all robots:".to_string(),
        format!("  max_accel = {} m/s²", b.max_accel),
        format!("  max_speed = {} m/s", b.max_speed),
        format!("  ramp_accel = {} m/s²", b.ramp_accel),
    ];
    for r in &b.robots {
        let mut own = Vec::new();
        for param in Param::ALL {
            if let Some(v) = r.get(param) {
                own.push(format!("{} = {v}", param.word()));
            }
        }
        lines.push(format!("{} {}: {}", r.team.word(), r.id, own.join(", ")));
    }
    lines
}

fn parse_param(word: &str) -> Result<Param, String> {
    Param::parse(word).ok_or_else(|| format!("unknown parameter `{word}` (max_accel, max_speed or ramp_accel)"))
}

fn parse_value(param: Param, word: &str) -> Result<f64, String> {
    word.parse::<f64>()
        .ok()
        .filter(|v| param.valid(*v))
        .ok_or_else(|| format!("`{word}` is not a number {}", param.rule()))
}

fn parse_robot(team: &str, id: &str) -> Result<(TeamColor, i32), String> {
    let team = TeamColor::parse(team).ok_or_else(|| format!("unknown team `{team}` (blue or yellow)"))?;
    let id = id.parse::<i32>().ok().filter(|id| (0..16).contains(id)).ok_or_else(|| format!("robot id `{id}` must be 0-15"))?;
    Ok((team, id))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> BangBangSettings {
        BangBangSettings {
            max_accel: 2.0,
            max_speed: 4.0,
            ramp_accel: 2.5,
            robots: vec![RobotBangBang { team: TeamColor::Yellow, id: 3, max_accel: Some(1.5), max_speed: None, ramp_accel: None }],
        }
    }

    #[test]
    fn robots_use_their_own_values_and_the_rest_from_all() {
        let s = settings();
        assert_eq!(s.for_robot(3, 1), BangBang { max_accel: 1.5, max_speed: 4.0, ramp_accel: 2.5 });
        assert_eq!(s.for_robot(3, 0), BangBang { max_accel: 2.0, max_speed: 4.0, ramp_accel: 2.5 });
    }

    #[test]
    fn file_round_trips() {
        let original = RobotSettings { bangbang: settings() };
        let text = toml::to_string(&original).unwrap();
        assert_eq!(parse(&text).unwrap(), original);
        // An empty file is the defaults.
        assert_eq!(parse("").unwrap(), RobotSettings::default());
    }

    #[test]
    fn rejects_limits_that_are_not_positive() {
        let mut s = RobotSettings { bangbang: settings() };
        s.bangbang.robots[0].max_speed = Some(0.0);
        assert!(s.validate().is_err());
        assert!(parse("[bangbang]\nmax_accel = -1\n").is_err());
    }
}
