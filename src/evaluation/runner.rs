// evaluation/runner.rs — Runs an evaluation in the background
//
// One headless engine process (this executable, `--headless`) per scenario and side, a few
// at a time; each runs lua/tests/evaluate.lua for N episodes and prints one line per
// episode, which updates the progress as it comes. `Run::cancel` kills the processes.

use std::collections::{HashMap, VecDeque};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use super::{EvalSettings, Episode, Evaluation, SideResults, EVAL_DIR, HARNESS};

/// Files the baseline takes from the working tree, so only the Lua differs.
const SHARED_FILES: [&str; 3] = [HARNESS, "simulator.toml", "robot_settings.toml"];
/// Extra simulated seconds a process may take beyond its episodes.
const TIME_SLACK: f64 = 60.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Baseline,
    Candidate,
}

impl Side {
    pub fn word(self) -> &'static str {
        match self {
            Side::Baseline => "baseline",
            Side::Candidate => "candidate",
        }
    }
}

/// What the panel shows while a run goes on, and its result.
#[derive(Debug, Clone, Default)]
pub struct Progress {
    pub phase: String,
    /// Episodes to play / played (cached baseline episodes are not counted).
    pub total: usize,
    pub done: usize,
    /// Set once the run ends.
    pub result: Option<Result<(Evaluation, PathBuf), String>>,
}

pub struct Run {
    progress: Arc<Mutex<Progress>>,
    cancel: Arc<AtomicBool>,
    children: Arc<Mutex<HashMap<u64, Child>>>,
}

impl Run {
    /// Starts evaluating `settings` in `root` (the repository root) with the engine
    /// executable `exe`.
    pub fn start(settings: EvalSettings, root: PathBuf, exe: PathBuf) -> Self {
        let run = Self {
            progress: Arc::new(Mutex::new(Progress { phase: "Starting".into(), ..Progress::default() })),
            cancel: Arc::new(AtomicBool::new(false)),
            children: Arc::new(Mutex::new(HashMap::new())),
        };
        let (progress, cancel, children) = (run.progress.clone(), run.cancel.clone(), run.children.clone());
        std::thread::spawn(move || {
            let result = evaluate(&settings, &root, &exe, &progress, &cancel, &children);
            let mut p = progress.lock().unwrap_or_else(|e| e.into_inner());
            p.phase = match &result {
                Ok(_) => "Done".into(),
                Err(e) if e == CANCELLED => "Cancelled".into(),
                Err(_) => "Failed".into(),
            };
            p.result = Some(result);
        });
        run
    }

    pub fn progress(&self) -> Progress {
        self.progress.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
        for child in self.children.lock().unwrap_or_else(|e| e.into_inner()).values_mut() {
            let _ = child.kill();
        }
    }
}

const CANCELLED: &str = "cancelled";

fn set_phase(progress: &Mutex<Progress>, phase: impl Into<String>) {
    progress.lock().unwrap_or_else(|e| e.into_inner()).phase = phase.into();
}

fn evaluate(
    settings: &EvalSettings,
    root: &Path,
    exe: &Path,
    progress: &Mutex<Progress>,
    cancel: &AtomicBool,
    children: &Arc<Mutex<HashMap<u64, Child>>>,
) -> Result<(Evaluation, PathBuf), String> {
    if settings.scenarios.is_empty() {
        return Err("no scenario selected".into());
    }
    if settings.repeats == 0 {
        return Err("episodes per scenario must be at least 1".into());
    }
    if !root.join(&settings.entry).is_file() {
        return Err(format!("entry script {} not found", settings.entry));
    }
    if !root.join(HARNESS).is_file() {
        return Err(format!("{HARNESS} not found in {}", root.display()));
    }

    set_phase(progress, format!("Exporting the baseline ({})", settings.baseline_ref));
    let commit = git(root, &["rev-parse", "--verify", &format!("{}^{{commit}}", settings.baseline_ref)])
        .map_err(|e| format!("baseline `{}`: {e}", settings.baseline_ref))?
        .trim()
        .to_string();
    let short = &commit[..commit.len().min(7)];
    let baseline_root = export_baseline(root, &commit)?;
    if !baseline_root.join(&settings.entry).is_file() {
        return Err(format!("{} does not exist at {}", settings.entry, settings.baseline_ref));
    }

    let key = cache_key(settings, root, &commit);
    let cache_path = root.join(EVAL_DIR).join("cache").join(format!("{key:016x}.json"));
    let cached: Option<SideResults> = std::fs::read_to_string(&cache_path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .filter(|side: &SideResults| complete(side, settings));

    let mut jobs: VecDeque<(Side, String)> = VecDeque::new();
    if cached.is_none() {
        jobs.extend(settings.scenarios.iter().map(|s| (Side::Baseline, s.clone())));
    }
    jobs.extend(settings.scenarios.iter().map(|s| (Side::Candidate, s.clone())));
    {
        let mut p = progress.lock().unwrap_or_else(|e| e.into_inner());
        p.total = jobs.len() * settings.repeats as usize;
        p.done = 0;
        p.phase = if cached.is_some() { "Running (baseline from cache)".into() } else { "Running".into() };
    }

    let baseline_label = format!("{} ({short})", settings.baseline_ref);
    let results = Mutex::new((
        cached.clone().unwrap_or(SideResults { label: baseline_label.clone(), episodes: Vec::new() }),
        SideResults { label: "working tree".into(), episodes: Vec::new() },
    ));
    let errors = Mutex::new(Vec::<String>::new());
    let jobs = Mutex::new(jobs);
    let next_id = AtomicU64::new(0);
    let workers = std::thread::available_parallelism().map(|n| n.get().saturating_sub(1)).unwrap_or(1).max(1);

    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| loop {
                if cancel.load(Ordering::SeqCst) {
                    return;
                }
                let Some((side, scenario)) = jobs.lock().unwrap_or_else(|e| e.into_inner()).pop_front() else { return };
                let side_root = match side {
                    Side::Baseline => baseline_root.as_path(),
                    Side::Candidate => root,
                };
                let outcome = run_scenario(settings, root, side_root, exe, &scenario, children, &next_id, cancel, |episode| {
                    let mut r = results.lock().unwrap_or_else(|e| e.into_inner());
                    match side {
                        Side::Baseline => r.0.episodes.push(episode),
                        Side::Candidate => r.1.episodes.push(episode),
                    }
                    progress.lock().unwrap_or_else(|e| e.into_inner()).done += 1;
                });
                if let Err(e) = outcome {
                    if !cancel.load(Ordering::SeqCst) {
                        errors.lock().unwrap_or_else(|e| e.into_inner()).push(format!("{} {scenario}: {e}", side.word()));
                    }
                }
            });
        }
    });

    if cancel.load(Ordering::SeqCst) {
        return Err(CANCELLED.into());
    }
    let errors = errors.into_inner().unwrap_or_else(|e| e.into_inner());
    if !errors.is_empty() {
        return Err(errors.join("\n"));
    }

    let (mut baseline, mut candidate) = results.into_inner().unwrap_or_else(|e| e.into_inner());
    baseline.label = baseline_label;
    for side in [&mut baseline, &mut candidate] {
        side.episodes.sort_by(|a, b| (&a.scenario, a.rep).cmp(&(&b.scenario, b.rep)));
    }
    if cached.is_none() {
        if let Some(dir) = cache_path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(text) = serde_json::to_string(&baseline) {
            let _ = std::fs::write(&cache_path, text);
        }
    }

    let created = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let evaluation = Evaluation { settings: settings.clone(), baseline, candidate, created };
    let path = evaluation.save(root)?;
    Ok((evaluation, path))
}

/// Whether `side` has every episode of `settings`.
fn complete(side: &SideResults, settings: &EvalSettings) -> bool {
    settings.scenarios.iter().all(|s| {
        (1..=settings.repeats).all(|rep| side.episodes.iter().any(|e| &e.scenario == s && e.rep == rep))
    })
}

/// Runs one scenario's episodes in a headless engine process in `side_root`, calling
/// `on_episode` for each line the harness prints.
#[allow(clippy::too_many_arguments)]
fn run_scenario(
    settings: &EvalSettings,
    root: &Path,
    side_root: &Path,
    exe: &Path,
    scenario: &str,
    children: &Mutex<HashMap<u64, Child>>,
    next_id: &AtomicU64,
    cancel: &AtomicBool,
    mut on_episode: impl FnMut(Episode),
) -> Result<(), String> {
    let scenario_path = root.join("scenarios").join(format!("{scenario}.toml"));
    if !scenario_path.is_file() {
        return Err(format!("{} not found", scenario_path.display()));
    }
    let max_time = settings.repeats as f64 * (settings.episode_time + 2.0) + TIME_SLACK;
    let mut command = Command::new(exe);
    command
        .args(["--headless", "--max-time", &format!("{max_time:.0}"), HARNESS])
        .current_dir(side_root)
        .env("EVAL_SCENARIO", &scenario_path)
        .env("EVAL_ENTRY", &settings.entry)
        .env("EVAL_N", settings.repeats.to_string())
        .env("EVAL_TIME", settings.episode_time.to_string())
        .env("EVAL_JITTER", settings.jitter.to_string())
        .env("EVAL_SEED", settings.seed.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = command.spawn().map_err(|e| format!("cannot start {}: {e}", exe.display()))?;
    let stdout = child.stdout.take().ok_or("no stdout")?;
    let mut stderr = child.stderr.take().ok_or("no stderr")?;
    let id = next_id.fetch_add(1, Ordering::SeqCst);
    children.lock().unwrap_or_else(|e| e.into_inner()).insert(id, child);
    if cancel.load(Ordering::SeqCst) {
        if let Some(mut c) = children.lock().unwrap_or_else(|e| e.into_inner()).remove(&id) {
            let _ = c.kill();
            let _ = c.wait();
        }
        return Err(CANCELLED.into());
    }

    // stderr in its own thread, so a chatty engine never blocks on a full pipe.
    let stderr_thread = std::thread::spawn(move || {
        let mut text = String::new();
        let _ = stderr.read_to_string(&mut text);
        text
    });

    let mut episodes = 0u32;
    for line in BufReader::new(stdout).lines().map_while(Result::ok) {
        if let Some(episode) = Episode::parse(scenario, &line) {
            episodes += 1;
            on_episode(episode);
        }
    }

    let status = children.lock().unwrap_or_else(|e| e.into_inner()).remove(&id).map(|mut c| c.wait());
    let stderr_text = stderr_thread.join().unwrap_or_default();
    if cancel.load(Ordering::SeqCst) {
        return Err(CANCELLED.into());
    }
    let code = match status {
        Some(Ok(status)) => status.code(),
        _ => None,
    };
    if code == Some(0) && episodes >= settings.repeats {
        return Ok(());
    }
    let tail: Vec<&str> = stderr_text.lines().rev().filter(|l| !l.trim().is_empty()).take(6).collect();
    let tail: Vec<&str> = tail.into_iter().rev().collect();
    Err(format!(
        "exit {} after {episodes}/{} episodes\n{}",
        code.map_or("?".to_string(), |c| c.to_string()),
        settings.repeats,
        tail.join("\n")
    ))
}

fn git(root: &Path, args: &[&str]) -> Result<String, String> {
    let mut command = Command::new("git");
    command.arg("-C").arg(root).args(args).stdin(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let out = command.output().map_err(|e| format!("cannot run git: {e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Writes the `lua/` tree of `commit` to sessions/eval/baseline-<commit>/ (once), plus the
/// working tree's shared files (every time, so both sides use the same).
fn export_baseline(root: &Path, commit: &str) -> Result<PathBuf, String> {
    let dir = root.join(EVAL_DIR).join(format!("baseline-{commit}"));
    let marker = dir.join(".complete");
    if !marker.exists() {
        let listing = git(root, &["ls-tree", "-r", "-z", commit, "--", "lua"])?;
        let mut blobs = Vec::new();
        for entry in listing.split('\0').filter(|e| !e.is_empty()) {
            let (meta, path) = entry.split_once('\t').ok_or_else(|| format!("bad ls-tree line `{entry}`"))?;
            let mut fields = meta.split_whitespace();
            let (_mode, kind, hash) = (fields.next(), fields.next(), fields.next());
            if kind == Some("blob") {
                blobs.push((hash.unwrap_or_default().to_string(), path.to_string()));
            }
        }
        if blobs.is_empty() {
            return Err(format!("no lua/ files at {commit}"));
        }
        let mut command = Command::new("git");
        command
            .arg("-C")
            .arg(root)
            .args(["cat-file", "--batch"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x0800_0000);
        }
        let mut child = command.spawn().map_err(|e| format!("cannot run git: {e}"))?;
        let mut stdin = child.stdin.take().ok_or("no stdin")?;
        let hashes: String = blobs.iter().map(|(h, _)| format!("{h}\n")).collect();
        let writer = std::thread::spawn(move || stdin.write_all(hashes.as_bytes()));
        let mut out = BufReader::new(child.stdout.take().ok_or("no stdout")?);
        for (_, path) in &blobs {
            let mut header = String::new();
            out.read_line(&mut header).map_err(|e| e.to_string())?;
            let size: usize = header
                .split_whitespace()
                .nth(2)
                .and_then(|s| s.parse().ok())
                .ok_or_else(|| format!("bad cat-file header `{}`", header.trim()))?;
            let mut content = vec![0u8; size + 1]; // content + '\n'
            out.read_exact(&mut content).map_err(|e| e.to_string())?;
            content.pop();
            let target = dir.join(path);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
            }
            std::fs::write(&target, content).map_err(|e| format!("cannot write {}: {e}", target.display()))?;
        }
        let _ = writer.join();
        let _ = child.wait();
        std::fs::write(&marker, commit).map_err(|e| e.to_string())?;
    }
    for file in SHARED_FILES {
        let from = root.join(file);
        let to = dir.join(file);
        if from.is_file() {
            if let Some(parent) = to.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            std::fs::copy(&from, &to).map_err(|e| format!("cannot copy {file}: {e}"))?;
        } else {
            let _ = std::fs::remove_file(&to);
        }
    }
    Ok(dir)
}

/// FNV-1a over everything the baseline's results depend on.
fn cache_key(settings: &EvalSettings, root: &Path, commit: &str) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    let mut feed = |bytes: &[u8]| {
        for &b in bytes.iter().chain([0xffu8].iter()) {
            h = (h ^ b as u64).wrapping_mul(0x0100_0000_01b3);
        }
    };
    feed(commit.as_bytes());
    feed(settings.entry.as_bytes());
    feed(&settings.repeats.to_le_bytes());
    feed(&settings.episode_time.to_bits().to_le_bytes());
    feed(&settings.jitter.to_bits().to_le_bytes());
    feed(&settings.seed.to_le_bytes());
    for file in SHARED_FILES {
        feed(&std::fs::read(root.join(file)).unwrap_or_default());
    }
    for scenario in &settings.scenarios {
        feed(scenario.as_bytes());
        feed(&std::fs::read(root.join("scenarios").join(format!("{scenario}.toml"))).unwrap_or_default());
    }
    // The engine build (its physics): size and modification time of the executable.
    if let Ok(meta) = std::env::current_exe().and_then(std::fs::metadata) {
        feed(&meta.len().to_le_bytes());
        let modified = meta.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok());
        feed(&modified.map_or(0, |d| d.as_secs()).to_le_bytes());
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_needs_every_episode() {
        let settings = EvalSettings { repeats: 2, scenarios: vec!["a".into()], ..EvalSettings::default() };
        let e = |rep| Episode { scenario: "a".into(), rep, result: "goal".into(), time: 1.0, kicks_blue: 0, kicks_yellow: 0 };
        assert!(!complete(&SideResults { label: String::new(), episodes: vec![e(1)] }, &settings));
        assert!(complete(&SideResults { label: String::new(), episodes: vec![e(2), e(1)] }, &settings));
    }

    /// Runs two scenarios × 3 episodes for HEAD against the working tree with the release
    /// engine. Slow (it plays the episodes): `cargo build --release`, then
    /// `cargo test --release -- --ignored evaluates_end_to_end`. With no Lua change since HEAD
    /// both sides must come out the same, episode by episode.
    #[test]
    #[ignore]
    fn evaluates_end_to_end() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let exe = root.join("target/release").join(if cfg!(windows) { "engine.exe" } else { "engine" });
        let settings = EvalSettings { repeats: 3, scenarios: vec!["2v2_wing".into(), "2vs1".into()], ..EvalSettings::default() };
        let run = Run::start(settings, root, exe);
        let (evaluation, path) = loop {
            if let Some(result) = run.progress().result {
                break result.expect("evaluation failed");
            }
            std::thread::sleep(std::time::Duration::from_millis(200));
        };
        assert_eq!(evaluation.baseline.episodes.len(), 6);
        assert_eq!(evaluation.candidate.episodes.len(), 6);
        let lua_changed = git(&evaluation_root(), &["status", "--porcelain", "--", "lua"]).map(|s| {
            s.lines().any(|l| !l.ends_with(HARNESS))
        });
        if lua_changed == Ok(false) {
            assert_eq!(evaluation.baseline.episodes, evaluation.candidate.episodes);
        }
        let summary = crate::evaluation::stats::summarize(&evaluation);
        assert_eq!(summary.overall.n, 6);
        assert!(path.is_file());
        let _ = std::fs::remove_file(path);
    }

    fn evaluation_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn the_cache_key_follows_the_settings() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let a = EvalSettings { scenarios: vec!["2v2_wing".into()], ..EvalSettings::default() };
        let b = EvalSettings { repeats: 11, ..a.clone() };
        assert_eq!(cache_key(&a, root, "abc"), cache_key(&a, root, "abc"));
        assert_ne!(cache_key(&a, root, "abc"), cache_key(&b, root, "abc"));
        assert_ne!(cache_key(&a, root, "abc"), cache_key(&a, root, "abd"));
    }
}
