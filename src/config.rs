// config.rs — Application startup configuration

use ini::Ini;
use std::fs;
use std::path::Path;

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
