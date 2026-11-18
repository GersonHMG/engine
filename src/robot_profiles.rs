// robot_profiles.rs — Saved robot profiles and generated variations of them
//
// A robot profile is the simulator's `[robot]` parameters (speed, grip, kicker,
// body), kept as a TOML file in `robots/` under the working directory (the
// repository root with `cargo run`). `sim robot load <name>` makes every
// simulated robot use one; the Robot editor panel edits and saves them.

use ssl_sim::{RobotConfig, SimConfig};
use std::fs;
use std::path::PathBuf;

pub const DIR: &str = "robots";

const HEADER: &str = "\
# Robot profile for the engine's simulator: the [robot] parameters of
# simulator.toml. Load it with `sim robot load <name>` in the Lua console or
# from the Robot editor panel. Missing values use the defaults.
";

/// Why `robot` cannot run in the simulator, if it cannot.
pub fn check(robot: &RobotConfig) -> Result<(), String> {
    SimConfig { robot: robot.clone(), ..SimConfig::default() }.validate()
}

/// A parameter's value, looked up by name.
pub fn value(robot: &RobotConfig, name: &str) -> Option<f64> {
    serde_json::to_value(robot).ok()?.get(name)?.as_f64()
}

/// Writes `robots/<name>.toml`, with `note` as comment lines under the
/// header. Returns the file written.
pub fn save(name: &str, robot: &RobotConfig, note: &str) -> Result<PathBuf, String> {
    let path = path_for(name)?;
    check(robot)?;
    fs::create_dir_all(DIR).map_err(|e| format!("cannot create {DIR}/: {e}"))?;
    fs::write(&path, to_text(robot, note)?).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    Ok(path)
}

pub fn load(name: &str) -> Result<RobotConfig, String> {
    let path = path_for(name)?;
    let text = fs::read_to_string(&path).map_err(|_| format!("no robot profile `{name}` ({} not found)", path.display()))?;
    let robot = from_text(&text).map_err(|e| format!("invalid {}: {e}", path.display()))?;
    check(&robot).map_err(|e| format!("invalid {}: {e}", path.display()))?;
    Ok(robot)
}

/// Removes `robots/<name>.toml`.
pub fn delete(name: &str) -> Result<(), String> {
    let path = path_for(name)?;
    fs::remove_file(&path).map_err(|e| format!("cannot delete {}: {e}", path.display()))
}

/// Names of the saved profiles, sorted.
pub fn list() -> Vec<String> {
    let Ok(entries) = fs::read_dir(DIR) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "toml"))
        .filter_map(|path| path.file_stem().map(|stem| stem.to_string_lossy().into_owned()))
        .collect();
    names.sort();
    names
}

/// Names are letters, digits, `-` and `_`, so they are safe file names.
pub fn valid_name(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn path_for(name: &str) -> Result<PathBuf, String> {
    if !valid_name(name) {
        return Err(format!("profile names use letters, digits, - and _, got `{name}`"));
    }
    Ok(PathBuf::from(DIR).join(format!("{name}.toml")))
}

fn to_text(robot: &RobotConfig, note: &str) -> Result<String, String> {
    let body = toml::to_string_pretty(robot).map_err(|e| e.to_string())?;
    let note: String = note.lines().filter(|l| !l.trim().is_empty()).map(|l| format!("# {l}\n")).collect();
    let note = if note.is_empty() { String::new() } else { format!("#\n{note}") };
    Ok(format!("{HEADER}{note}\n{body}"))
}

fn from_text(text: &str) -> Result<RobotConfig, String> {
    toml::from_str(text).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_text_loads_back() {
        let robot = RobotConfig { max_speed: 2.75, grip_accel: 4.0, ..RobotConfig::default() };
        let text = to_text(&robot, "seed 3\nmax_speed ±10%").unwrap();
        assert!(text.contains("# seed 3\n"), "{text}");
        assert_eq!(from_text(&text), Ok(robot));
    }

    #[test]
    fn names_must_be_safe_file_names() {
        assert!(path_for("fast_grippy-02").is_ok());
        assert!(path_for("").is_err());
        assert!(path_for("../simulator").is_err());
    }
}
