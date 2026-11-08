// evaluation — Is a Lua change better than the baseline? Scenario statistics
//
// The Evaluation panel runs every chosen scenario (scenarios/*.toml) N times with
// lua/tests/evaluate.lua, in headless engine processes, twice:
//   * candidate: the working tree's Lua (what you are editing);
//   * baseline: the Lua of a git commit (HEAD by default), exported to
//     sessions/eval/baseline-<commit>/ and run with the working tree's harness,
//     scenarios, simulator.toml and robot_settings.toml, so only the Lua differs.
// Both sides play the same jittered starts, and `stats` compares them pair by pair.
// Baseline results are cached per commit and settings (sessions/eval/cache/), and every
// finished evaluation is saved to sessions/eval/<unix time>.json.

pub mod runner;
pub mod stats;

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Where evaluations keep their files (git-ignored, under the working directory).
pub const EVAL_DIR: &str = "sessions/eval";
/// The harness, relative to a side's root.
pub const HARNESS: &str = "lua/tests/evaluate.lua";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvalSettings {
    /// Entry script, relative to the repository root.
    pub entry: String,
    /// Git revision of the baseline.
    pub baseline_ref: String,
    /// Episodes per scenario and side.
    pub repeats: u32,
    /// s per episode.
    pub episode_time: f64,
    /// m (and 2× rad for headings) of start jitter, episodes 2..N.
    pub jitter: f64,
    pub seed: u64,
    /// Scenario names (scenarios/<name>.toml).
    pub scenarios: Vec<String>,
}

impl Default for EvalSettings {
    fn default() -> Self {
        Self {
            entry: "lua/run_ai.lua".into(),
            baseline_ref: "HEAD".into(),
            repeats: 10,
            episode_time: 15.0,
            jitter: 0.05,
            seed: 1,
            scenarios: Vec::new(),
        }
    }
}

/// One episode's outcome (a line of lua/tests/evaluate.lua).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Episode {
    pub scenario: String,
    pub rep: u32,
    /// goal, conceded, foul, out, cleared, saved, stolen, own_area, timeout.
    pub result: String,
    /// s from the start to the result.
    pub time: f64,
    pub kicks_blue: u32,
    pub kicks_yellow: u32,
}

impl Episode {
    pub fn won(&self) -> bool {
        self.result == "goal"
    }

    /// Parses `EPISODE <rep> <result> <time> <blue kicks> <yellow kicks>`.
    pub fn parse(scenario: &str, line: &str) -> Option<Self> {
        let mut parts = line.trim_end().split('\t');
        if parts.next()? != "EPISODE" {
            return None;
        }
        Some(Self {
            scenario: scenario.to_string(),
            rep: parts.next()?.parse().ok()?,
            result: parts.next()?.to_string(),
            time: parts.next()?.parse().ok()?,
            kicks_blue: parts.next()?.parse().ok()?,
            kicks_yellow: parts.next()?.parse().ok()?,
        })
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SideResults {
    /// "working tree", "HEAD (1a2b3c4)".
    pub label: String,
    pub episodes: Vec<Episode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evaluation {
    pub settings: EvalSettings,
    pub baseline: SideResults,
    pub candidate: SideResults,
    /// Unix time, s, when it finished.
    pub created: u64,
}

impl Evaluation {
    pub fn save(&self, root: &Path) -> Result<PathBuf, String> {
        let dir = root.join(EVAL_DIR);
        std::fs::create_dir_all(&dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
        let path = dir.join(format!("{}.json", self.created));
        let text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(&path, text).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
        Ok(path)
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
    }
}

/// The scenario names in `root`/scenarios, sorted.
pub fn scenario_names(root: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(root.join("scenarios"))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension()? != "toml" {
                return None;
            }
            Some(path.file_stem()?.to_string_lossy().into_owned())
        })
        .collect();
    names.sort();
    names
}

/// The most recent saved evaluation in `root`, if any.
pub fn latest_saved(root: &Path) -> Option<PathBuf> {
    std::fs::read_dir(root.join(EVAL_DIR))
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .filter(|p| p.file_stem().and_then(|s| s.to_str()).is_some_and(|s| s.parse::<u64>().is_ok()))
        .max()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_harness_lines() {
        let e = Episode::parse("wing", "EPISODE\t3\tgoal\t3.483\t2\t0\r\n").unwrap();
        assert_eq!(e, Episode { scenario: "wing".into(), rep: 3, result: "goal".into(), time: 3.483, kicks_blue: 2, kicks_yellow: 0 });
        assert!(e.won());
        assert!(Episode::parse("wing", "DONE\t3").is_none());
        assert!(Episode::parse("wing", "EPISODE\tx\tgoal\t1\t0\t0").is_none());
    }
}
