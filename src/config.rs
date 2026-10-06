// config.rs — Application startup configuration

use ini::Ini;
use ssl_sim::SimConfig;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// The built-in simulator's physics parameters (see `simulator.toml`).
const SIMULATOR_CONFIG_FILE: &str = "simulator.toml";

const SIMULATOR_CONFIG_HEADER: &str = "\
# Parameters of the engine's built-in simulator (SIM button).
# Read every time SIM is turned on; `sim save` in the Lua console rewrites
# this file with the current values. Missing values use the defaults.
# Field length and width come from config.ini.
#
# Units: meters, seconds, radians (m/s, m/s^2, rad/s, rad/s^2).
# Restitutions go from 0 (no bounce) to 1 (perfect bounce).
";

const DEFAULT_CONFIG_TEXT: &str = "; Sysmic Engine configuration\n\n[field]\n; Field length/width in meters\nlength_m = 9.0\nwidth_m = 6.0\n";

#[derive(Debug, Clone, Copy)]
pub struct FieldConfig {
    pub length_m: f64,
    pub width_m: f64,
}

impl Default for FieldConfig {
    fn default() -> Self {
        Self {
            length_m: 9.0,
            width_m: 6.0,
        }
    }
}

pub fn load_field_config<P: AsRef<Path>>(path: P) -> FieldConfig {
    let mut config = FieldConfig::default();
    let ini = match Ini::load_from_file(path) {
        Ok(ini) => ini,
        Err(_) => return config,
    };

    if let Some(section) = ini.section(Some("field")) {
        if let Some(value) = section.get("length_m") {
            if let Ok(parsed) = value.trim().parse::<f64>() {
                if parsed > 0.0 {
                    config.length_m = parsed;
                }
            }
        }

        if let Some(value) = section.get("width_m") {
            if let Ok(parsed) = value.trim().parse::<f64>() {
                if parsed > 0.0 {
                    config.width_m = parsed;
                }
            }
        }
    }

    config
}

pub fn load_field_config_from_exe() -> FieldConfig {
    let exe_dir = match std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|p| p.to_path_buf()))
    {
        Some(dir) => dir,
        None => return load_field_config("config.ini"),
    };

    let exe_config = exe_dir.join("config.ini");

    if !exe_config.exists() {
        let cwd_config = Path::new("config.ini");
        if cwd_config.exists() {
            let _ = fs::copy(cwd_config, &exe_config);
        } else {
            let _ = fs::write(&exe_config, DEFAULT_CONFIG_TEXT);
        }
    }

    if !exe_config.exists() {
        return load_field_config("config.ini");
    }

    load_field_config(exe_config)
}

/// Where `simulator.toml` is read from and saved to: the working directory
/// (the repository root with `cargo run`) or, failing that, next to the
/// executable. A new file goes in the working directory.
pub fn simulator_config_path() -> PathBuf {
    let in_working_dir = PathBuf::from(SIMULATOR_CONFIG_FILE);
    if in_working_dir.exists() {
        return in_working_dir;
    }
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join(SIMULATOR_CONFIG_FILE)))
        .filter(|next_to_exe| next_to_exe.exists())
        .unwrap_or(in_working_dir)
}

/// The simulator parameters from `simulator.toml`, with the field size from
/// config.ini. Missing values keep their defaults; a missing or invalid file
/// means all defaults.
pub fn load_simulator_config(field: FieldConfig) -> SimConfig {
    let path = simulator_config_path();
    let loaded = match fs::read_to_string(&path) {
        Ok(text) => parse_simulator_config(&text, field)
            .map_err(|e| warn!("Invalid {}: {e}. Using simulator defaults.", path.display()))
            .ok(),
        Err(_) => {
            info!("{} not found, using simulator defaults", path.display());
            None
        }
    };

    match loaded {
        Some(config) => {
            info!("Simulator parameters loaded from {}", path.display());
            config
        }
        None => with_field_size(SimConfig::default(), field),
    }
}

/// Writes the simulator parameters to `simulator.toml`, without the field
/// size (config.ini owns it). Returns the file written.
pub fn save_simulator_config(config: &SimConfig) -> Result<PathBuf, String> {
    let path = simulator_config_path();
    fs::write(&path, simulator_config_text(config)?)
        .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    Ok(path)
}

fn parse_simulator_config(text: &str, field: FieldConfig) -> Result<SimConfig, String> {
    let config: SimConfig = toml::from_str(text).map_err(|e| e.to_string())?;
    let config = with_field_size(config, field);
    config.validate()?;
    Ok(config)
}

fn simulator_config_text(config: &SimConfig) -> Result<String, String> {
    let mut table = toml::Table::try_from(config).map_err(|e| e.to_string())?;
    if let Some(toml::Value::Table(field)) = table.get_mut("field") {
        field.remove("length");
        field.remove("width");
    }
    let body = toml::to_string_pretty(&table).map_err(|e| e.to_string())?;
    Ok(format!("{SIMULATOR_CONFIG_HEADER}\n{body}"))
}

fn with_field_size(mut config: SimConfig, field: FieldConfig) -> SimConfig {
    config.field.length = field.length_m;
    config.field.width = field.width_m;
    config
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_text_loads_back_unchanged() {
        let mut config = with_field_size(SimConfig::default(), FieldConfig::default());
        config.robot.max_speed = 2.25;
        config.ball.roll_decel = 0.5;

        let text = simulator_config_text(&config).unwrap();
        assert!(!text.contains("length ="), "field size belongs to config.ini:\n{text}");
        assert_eq!(parse_simulator_config(&text, FieldConfig::default()), Ok(config));
    }

    #[test]
    fn missing_values_keep_their_defaults() {
        let config = parse_simulator_config("[robot]\nmax_speed = 2.0\n", FieldConfig::default()).unwrap();
        assert_eq!(config.robot.max_speed, 2.0);
        assert_eq!(config.ball, SimConfig::default().ball);
    }

    #[test]
    fn rejects_unknown_and_invalid_values() {
        let field = FieldConfig::default();
        assert!(parse_simulator_config("[robot]\nspeed = 2.0\n", field).is_err());
        assert!(parse_simulator_config("max_substep = 0.0\n", field).is_err());
    }

    /// The repository's simulator.toml must stay loadable.
    #[test]
    fn repository_file_loads() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(SIMULATOR_CONFIG_FILE);
        let text = fs::read_to_string(path).unwrap();
        parse_simulator_config(&text, FieldConfig::default()).unwrap();
    }
}
