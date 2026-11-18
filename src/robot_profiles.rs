// robot_profiles.rs — Saved robot profiles and generated variations of them
//
// A robot profile is the simulator's `[robot]` parameters (speed, grip, kicker,
// body), kept as a TOML file in `robots/` under the working directory (the
// repository root with `cargo run`). `sim robot load <name>` makes every
// simulated robot use one; the Robot variations panel generates and saves them.
//
// Variations spread each chosen parameter by ±a fraction of the base value,
// sampled as a Latin hypercube: with N variations, every parameter's range is
// cut into N equal slices and each slice is used exactly once, so even a few
// variations cover the whole range instead of clumping by chance.

use ssl_sim::{RobotConfig, SimConfig};
use std::fs;
use std::path::PathBuf;

pub const DIR: &str = "robots";

const HEADER: &str = "\
# Robot profile for the engine's simulator: the [robot] parameters of
# simulator.toml. Load it with `sim robot load <name>` in the Lua console or
# from the Robot variations panel. Missing values use the defaults.
";

/// How much one parameter may move away from the base: ±`fraction` × base.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spread {
    pub name: &'static str,
    pub fraction: f64,
}

/// `count` variations of `base`, reproducible from `seed`. Parameters not in
/// `spreads` keep the base value. Values are rounded to 4 significant digits
/// and kept physically valid (fractions in [0, 1], front face inside the body).
pub fn generate(base: &RobotConfig, spreads: &[Spread], count: usize, seed: u64) -> Vec<RobotConfig> {
    let mut rng = Rng(seed ^ 0x5EED_0F_B07);
    let Ok(serde_json::Value::Object(base_fields)) = serde_json::to_value(base) else {
        return Vec::new();
    };

    // One shuffled list of slices per parameter (the Latin hypercube).
    let slices: Vec<Vec<usize>> = spreads
        .iter()
        .map(|_| {
            let mut order: Vec<usize> = (0..count).collect();
            for i in (1..count).rev() {
                order.swap(i, rng.below(i + 1));
            }
            order
        })
        .collect();

    (0..count)
        .map(|k| {
            let mut fields = base_fields.clone();
            for (spread, order) in spreads.iter().zip(&slices) {
                let Some(base_value) = fields.get(spread.name).and_then(|v| v.as_f64()) else {
                    continue;
                };
                let u = (order[k] as f64 + rng.unit()) / count as f64; // in [0, 1)
                let factor = 1.0 + spread.fraction.max(0.0) * (2.0 * u - 1.0);
                let mut value = round_significant((base_value * factor).max(0.0), 4);
                if is_fraction(spread.name) {
                    value = value.min(1.0);
                }
                fields.insert(spread.name.to_string(), serde_json::json!(value));
            }
            let mut robot: RobotConfig =
                serde_json::from_value(serde_json::Value::Object(fields)).unwrap_or_else(|_| base.clone());
            // The flat front face must stay inside the round body.
            if robot.front_distance >= robot.radius {
                robot.front_distance = round_significant(robot.radius * 0.95, 4);
            }
            robot
        })
        .collect()
}

/// Parameters that must stay in [0, 1].
pub fn is_fraction(name: &str) -> bool {
    matches!(name, "slide_grip" | "body_restitution" | "front_restitution")
}

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

fn round_significant(v: f64, digits: i32) -> f64 {
    if v == 0.0 || !v.is_finite() {
        return v;
    }
    let scale = 10f64.powi(digits - 1 - v.abs().log10().floor() as i32);
    (v * scale).round() / scale
}

/// SplitMix64: small, seeded, reproducible.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in [0, 1).
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spreads() -> Vec<Spread> {
        vec![
            Spread { name: "max_speed", fraction: 0.2 },
            Spread { name: "front_restitution", fraction: 0.5 },
            Spread { name: "front_distance", fraction: 0.3 },
        ]
    }

    #[test]
    fn variations_stay_in_range_and_valid() {
        let base = RobotConfig { front_restitution: 0.9, ..RobotConfig::default() };
        let variations = generate(&base, &spreads(), 40, 7);
        assert_eq!(variations.len(), 40);
        for v in &variations {
            assert!(check(v).is_ok(), "{v:?}");
            assert!((base.max_speed * 0.8..=base.max_speed * 1.2).contains(&v.max_speed));
            assert!(v.front_restitution <= 1.0);
            assert_eq!(v.max_accel, base.max_accel, "parameters without a spread keep the base value");
        }
    }

    #[test]
    fn latin_hypercube_uses_every_slice_once() {
        let base = RobotConfig::default();
        let count = 10;
        let spread = [Spread { name: "max_speed", fraction: 0.5 }];
        let mut slices: Vec<usize> = generate(&base, &spread, count, 3)
            .iter()
            .map(|v| {
                let u = (v.max_speed / base.max_speed - 0.5) / 1.0; // back to [0, 1)
                ((u * count as f64).floor() as usize).min(count - 1)
            })
            .collect();
        slices.sort();
        assert_eq!(slices, (0..count).collect::<Vec<_>>());
    }

    #[test]
    fn same_seed_same_variations() {
        let base = RobotConfig::default();
        assert_eq!(generate(&base, &spreads(), 5, 1), generate(&base, &spreads(), 5, 1));
        assert_ne!(generate(&base, &spreads(), 5, 1), generate(&base, &spreads(), 5, 2));
    }

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
