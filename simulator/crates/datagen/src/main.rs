//! `engine-datagen`: robot motion datasets for learning the simulator's dynamics.
//!
//! One robot, no walls (the field is made huge), the ball far away. Each episode
//! starts after a random warm-up (so the robot is already moving) and then drives
//! the robot with a random command signal, one command per control tick. Every
//! tick writes the state *before* the step together with the command applied in
//! that step, so row k and row k+1 of an episode form one transition
//! (s_k, u_k) -> s_{k+1}. The last row of an episode has no command (NaN).
//!
//! Output: `<out>.npy` (float64, one row per tick, columns in `COLUMNS`) and
//! `<out>.json` (seed, mode, physics config, simulator timing).
//!
//! Faulty robots: `--fault-matrix m00,m01,...,m22` makes the robot execute
//! M·(vt, vn, ω) instead of the command it receives (a weak wheel, a robot that
//! curves when it strafes, ...). The file still records the command *sent*,
//! as a log of a real faulty robot would.
//!
//! ```sh
//! cargo run -q --release -p ssl-sim-datagen -- --mode mix --episodes 2000 --seconds 10 --seed 1 --out data/train
//! ```

use anyhow::{bail, Context, Result};
use ssl_sim::{wrap_angle, Ball, Robot, RobotCommand, RobotConfig, RobotId, SimConfig, SimState, Simulator, Team, Vec2};
use std::f64::consts::PI;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::time::Instant;

const COLUMNS: [&str; 12] = [
    "episode", "tick", "seg", "vt_cmd", "vn_cmd", "w_cmd", "x", "y", "theta", "vx", "vy", "omega",
];
const TICK: f64 = 1.0 / 60.0;
const ROBOT: RobotId = RobotId::new(Team::Blue, 0);

/// Segment kinds, stored in the `seg` column.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Seg {
    Step = 0,
    Multisine = 1,
    Noise = 2,
    Chirp = 3,
    Idle = 4,
    Waypoint = 5,
}

const SEG_NAMES: [&str; 6] = ["step", "multisine", "noise", "chirp", "idle", "waypoint"];

// ─── Random numbers (SplitMix64: tiny, deterministic, no dependency) ────────

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in [0, 1).
    fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    fn uniform(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.unit()
    }

    fn chance(&mut self, p: f64) -> bool {
        self.unit() < p
    }

    fn log_uniform(&mut self, lo: f64, hi: f64) -> f64 {
        (lo.ln() + (hi.ln() - lo.ln()) * self.unit()).exp()
    }
}

/// Independent stream per (seed, episode), so episodes do not depend on each other.
fn episode_rng(seed: u64, episode: u64) -> Rng {
    let mut mix = Rng::new(seed ^ 0xD1B5_4A32_D192_ED03);
    let a = mix.next_u64();
    Rng::new(a ^ episode.wrapping_mul(0xA24B_AED4_963E_E407))
}

// ─── Command signals ────────────────────────────────────────────────────────

/// A source of commands. `tick` is called once per control tick with the
/// robot's state before the step and returns the robot-frame command.
trait Signal {
    fn seg(&self) -> Seg;
    fn done(&self) -> bool;
    fn tick(&mut self, robot: &Robot) -> RobotCommand;
}

fn command(vt: f64, vn: f64, omega: f64) -> RobotCommand {
    RobotCommand { vt, vn, omega, kick_speed: 0.0 }
}

/// Piecewise-constant command held for a random time; up to 1.3x the limits.
struct Step {
    left: usize,
    cmd: RobotCommand,
}

impl Step {
    fn new(rng: &mut Rng, lim: &RobotConfig, min_s: f64, max_s: f64) -> Self {
        let speed = if rng.chance(0.15) { 0.0 } else { rng.uniform(0.0, 1.3 * lim.max_speed) };
        let dir = rng.uniform(-PI, PI);
        let omega = if rng.chance(0.25) { 0.0 } else { rng.uniform(-1.3, 1.3) * lim.max_omega };
        Self {
            left: ticks(rng.uniform(min_s, max_s)),
            cmd: command(speed * dir.cos(), speed * dir.sin(), omega),
        }
    }
}

impl Signal for Step {
    fn seg(&self) -> Seg {
        Seg::Step
    }
    fn done(&self) -> bool {
        self.left == 0
    }
    fn tick(&mut self, _: &Robot) -> RobotCommand {
        self.left -= 1;
        self.cmd
    }
}

/// Zero command: the robot brakes to rest.
struct Idle {
    left: usize,
}

impl Signal for Idle {
    fn seg(&self) -> Seg {
        Seg::Idle
    }
    fn done(&self) -> bool {
        self.left == 0
    }
    fn tick(&mut self, _: &Robot) -> RobotCommand {
        self.left -= 1;
        RobotCommand::default()
    }
}

/// One smooth channel: a sum of sinusoids, a chirp or low-pass filtered noise,
/// in [-1, 1] before scaling.
enum Wave {
    Multisine { parts: Vec<(f64, f64, f64)> }, // (weight, freq Hz, phase)
    Chirp { f0: f64, f1: f64, duration: f64, phase: f64 },
    Noise { state: f64, alpha: f64, gain: f64 },
}

impl Wave {
    fn new(rng: &mut Rng, kind: Seg, duration: f64) -> Self {
        match kind {
            Seg::Multisine => {
                let n = 1 + (rng.unit() * 3.0) as usize;
                let mut parts: Vec<(f64, f64, f64)> = (0..n)
                    .map(|_| (rng.unit(), rng.log_uniform(0.1, 3.0), rng.uniform(-PI, PI)))
                    .collect();
                let total: f64 = parts.iter().map(|p| p.0).sum::<f64>().max(1e-9);
                parts.iter_mut().for_each(|p| p.0 /= total);
                Wave::Multisine { parts }
            }
            Seg::Chirp => {
                let (a, b) = (rng.log_uniform(0.1, 3.0), rng.log_uniform(0.1, 3.0));
                Wave::Chirp { f0: a, f1: b, duration, phase: rng.uniform(-PI, PI) }
            }
            _ => {
                // First-order low-pass of uniform white noise, cutoff 0.3-3 Hz.
                let cutoff = rng.log_uniform(0.3, 3.0);
                let alpha = 1.0 - (-2.0 * PI * cutoff * TICK).exp();
                // Rough gain so the filtered signal spans about [-1, 1].
                let gain = (2.0 / alpha).sqrt();
                Wave::Noise { state: rng.uniform(-1.0, 1.0) / gain, alpha, gain }
            }
        }
    }

    fn value(&mut self, rng: &mut Rng, t: f64) -> f64 {
        match self {
            Wave::Multisine { parts } => parts.iter().map(|(w, f, ph)| w * (2.0 * PI * f * t + ph).sin()).sum(),
            Wave::Chirp { f0, f1, duration, phase } => {
                let k = (*f1 - *f0) / *duration;
                (2.0 * PI * (*f0 * t + 0.5 * k * t * t) + *phase).sin()
            }
            Wave::Noise { state, alpha, gain } => {
                *state += *alpha * (rng.uniform(-1.0, 1.0) - *state);
                (*state * *gain).clamp(-1.0, 1.0)
            }
        }
    }
}

/// Three smooth channels (vt, vn, ω) of the same kind.
struct Smooth {
    kind: Seg,
    left: usize,
    t: f64,
    waves: [Wave; 3],
    amps: [f64; 3],
    offsets: [f64; 3],
    rng: Rng,
}

impl Smooth {
    fn new(rng: &mut Rng, lim: &RobotConfig, kind: Seg) -> Self {
        let duration = rng.uniform(1.0, 3.0);
        let mut channel = |limit: f64| {
            let amp = if rng.chance(0.15) { 0.0 } else { rng.uniform(0.0, 1.0) * limit };
            let offset = rng.uniform(-0.3, 0.3) * limit;
            (Wave::new(rng, kind, duration), amp, offset)
        };
        let (w0, a0, o0) = channel(lim.max_speed);
        let (w1, a1, o1) = channel(lim.max_speed);
        let (w2, a2, o2) = channel(1.3 * lim.max_omega);
        Self {
            kind,
            left: ticks(duration),
            t: 0.0,
            waves: [w0, w1, w2],
            amps: [a0, a1, a2],
            offsets: [o0, o1, o2],
            rng: Rng::new(rng.next_u64()),
        }
    }
}

impl Signal for Smooth {
    fn seg(&self) -> Seg {
        self.kind
    }
    fn done(&self) -> bool {
        self.left == 0
    }
    fn tick(&mut self, _: &Robot) -> RobotCommand {
        let mut v = [0.0; 3];
        for i in 0..3 {
            v[i] = self.offsets[i] + self.amps[i] * self.waves[i].value(&mut self.rng, self.t);
        }
        self.left -= 1;
        self.t += TICK;
        command(v[0], v[1], v[2])
    }
}

/// A simple go-to-point controller, like an AI would drive: decelerates on
/// approach, turns toward a heading, never commands above the limits.
struct Waypoint {
    target: Vec2,
    heading: f64,
    vcap: f64,
    accel: f64,
    wcap: f64,
    left: usize,
    rng: Rng,
}

impl Waypoint {
    fn new(rng: &mut Rng, lim: &RobotConfig, from: &Robot) -> Self {
        let dist = rng.uniform(0.5, 4.0);
        let dir = rng.uniform(-PI, PI);
        let target = from.position + Vec2::new(dist * dir.cos(), dist * dir.sin());
        let heading = if rng.chance(0.5) { dir } else { rng.uniform(-PI, PI) };
        Self {
            target,
            heading,
            vcap: rng.uniform(0.3, 0.9) * lim.max_speed,
            accel: rng.uniform(0.3, 0.9) * lim.max_accel,
            wcap: rng.uniform(0.2, 0.8) * lim.max_omega,
            left: ticks(5.0),
            rng: Rng::new(rng.next_u64()),
        }
    }
}

impl Signal for Waypoint {
    fn seg(&self) -> Seg {
        Seg::Waypoint
    }
    fn done(&self) -> bool {
        self.left == 0
    }
    fn tick(&mut self, robot: &Robot) -> RobotCommand {
        let offset = self.target - robot.position;
        let dist = offset.length();
        let heading_error = wrap_angle(self.heading - robot.theta);
        if dist < 0.03 && heading_error.abs() < 0.05 {
            // Arrived: hold still for a moment, then finish.
            self.left = self.left.min(ticks(self.rng.uniform(0.0, 0.3)));
        }
        let speed = self.vcap.min((2.0 * self.accel * dist).sqrt());
        let world = if dist > 0.01 { offset * (speed / dist) } else { Vec2::ZERO };
        let local = world.rotated(-robot.theta);
        let omega = (4.0 * heading_error).clamp(-self.wcap, self.wcap);
        self.left = self.left.saturating_sub(1);
        command(local.x, local.y, omega)
    }
}

fn ticks(seconds: f64) -> usize {
    ((seconds / TICK).round() as usize).max(1)
}

/// Picks the next segment for the training mix.
fn next_mix(rng: &mut Rng, lim: &RobotConfig) -> Box<dyn Signal> {
    let r = rng.unit();
    if r < 0.40 {
        Box::new(Step::new(rng, lim, 0.05, 1.5))
    } else if r < 0.60 {
        Box::new(Smooth::new(rng, lim, Seg::Multisine))
    } else if r < 0.75 {
        Box::new(Smooth::new(rng, lim, Seg::Noise))
    } else if r < 0.85 {
        Box::new(Smooth::new(rng, lim, Seg::Chirp))
    } else {
        Box::new(Idle { left: ticks(rng.uniform(0.2, 1.0)) })
    }
}

// ─── Episodes ───────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Mix,
    Waypoint,
}

/// What the robot executes for a command: M·(vt, vn, ω); identity for a healthy robot.
#[derive(Clone, Copy)]
struct Fault([[f64; 3]; 3]);

impl Fault {
    const NONE: Fault = Fault([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]);

    fn apply(&self, c: RobotCommand) -> RobotCommand {
        let m = &self.0;
        let u = [c.vt, c.vn, c.omega];
        let row = |r: &[f64; 3]| r[0] * u[0] + r[1] * u[1] + r[2] * u[2];
        RobotCommand { vt: row(&m[0]), vn: row(&m[1]), omega: row(&m[2]), kick_speed: c.kick_speed }
    }
}

fn robot(sim: &Simulator) -> &Robot {
    sim.state().robot(ROBOT).expect("the episode robot exists")
}

/// Runs one episode and appends its rows. Returns the number of simulator steps.
fn episode(
    config: &SimConfig,
    mode: Mode,
    fault: Fault,
    seed: u64,
    index: u64,
    ticks_per_ep: usize,
    rows: &mut Vec<f64>,
) -> usize {
    let mut rng = episode_rng(seed, index);
    let lim = config.robot.clone();
    let theta = rng.uniform(-PI, PI);
    let state = SimState::new(
        vec![Robot::new(ROBOT, Vec2::ZERO, theta)],
        // Far from the robot's reach: the ball never touches it.
        Ball::new(Vec2::new(3000.0, 3000.0), Vec2::ZERO),
    );
    let mut sim = Simulator::new(config.clone(), state);
    let mut steps = 0;

    // Warm-up, not recorded: start each episode already moving.
    let mut warm = Step::new(&mut rng, &lim, 0.0, 1.0);
    if mode == Mode::Waypoint {
        warm.cmd = command(warm.cmd.vt * 0.6, warm.cmd.vn * 0.6, warm.cmd.omega * 0.5);
    }
    while !warm.done() {
        let cmd = warm.tick(robot(&sim));
        sim.set_command(ROBOT, fault.apply(cmd)).expect("robot exists");
        sim.step(TICK);
        steps += 1;
    }

    let mut signal: Box<dyn Signal> = match mode {
        Mode::Mix => next_mix(&mut rng, &lim),
        Mode::Waypoint => Box::new(Waypoint::new(&mut rng, &lim, robot(&sim))),
    };
    for tick in 0..=ticks_per_ep {
        if signal.done() {
            signal = match mode {
                Mode::Mix => next_mix(&mut rng, &lim),
                Mode::Waypoint => Box::new(Waypoint::new(&mut rng, &lim, robot(&sim))),
            };
        }
        let r = robot(&sim).clone();
        let (cmd, seg) = if tick < ticks_per_ep {
            let cmd = signal.tick(&r);
            (Some(cmd), signal.seg())
        } else {
            (None, signal.seg())
        };
        let (vt, vn, w) = cmd.map_or((f64::NAN, f64::NAN, f64::NAN), |c| (c.vt, c.vn, c.omega));
        rows.extend_from_slice(&[
            index as f64,
            tick as f64,
            seg as i32 as f64,
            vt,
            vn,
            w,
            r.position.x,
            r.position.y,
            r.theta,
            r.velocity.x,
            r.velocity.y,
            r.omega,
        ]);
        if let Some(cmd) = cmd {
            sim.set_command(ROBOT, fault.apply(cmd)).expect("robot exists");
            sim.step(TICK);
            steps += 1;
        }
    }
    steps
}

// ─── Output ─────────────────────────────────────────────────────────────────

/// Writes a C-order float64 array in the .npy v1.0 format.
fn write_npy(path: &str, data: &[f64], cols: usize) -> Result<()> {
    let rows = data.len() / cols;
    let mut header = format!("{{'descr': '<f8', 'fortran_order': False, 'shape': ({rows}, {cols}), }}");
    // magic (6) + version (2) + length (2) + header + '\n' must be a multiple of 64.
    let pad = 64 - (10 + header.len() + 1) % 64;
    header.push_str(&" ".repeat(pad % 64));
    header.push('\n');

    let file = File::create(path).with_context(|| format!("cannot create {path}"))?;
    let mut out = BufWriter::new(file);
    out.write_all(b"\x93NUMPY\x01\x00")?;
    out.write_all(&(header.len() as u16).to_le_bytes())?;
    out.write_all(header.as_bytes())?;
    for value in data {
        out.write_all(&value.to_le_bytes())?;
    }
    out.flush()?;
    Ok(())
}

struct Args {
    mode: Mode,
    episodes: u64,
    seconds: f64,
    seed: u64,
    out: String,
    fault: Fault,
    fault_name: String,
}

fn parse_args() -> Result<Args> {
    let mut args = Args {
        mode: Mode::Mix,
        episodes: 100,
        seconds: 10.0,
        seed: 0,
        out: String::new(),
        fault: Fault::NONE,
        fault_name: "none".into(),
    };
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        let mut value = || it.next().with_context(|| format!("{flag} needs a value"));
        match flag.as_str() {
            "--mode" => {
                args.mode = match value()?.as_str() {
                    "mix" => Mode::Mix,
                    "waypoint" => Mode::Waypoint,
                    other => bail!("unknown mode {other} (mix, waypoint)"),
                }
            }
            "--episodes" => args.episodes = value()?.parse()?,
            "--seconds" => args.seconds = value()?.parse()?,
            "--seed" => args.seed = value()?.parse()?,
            "--out" => args.out = value()?,
            "--fault-name" => args.fault_name = value()?,
            "--fault-matrix" => {
                let v: Vec<f64> = value()?.split(',').map(|x| x.trim().parse()).collect::<Result<_, _>>()?;
                if v.len() != 9 || v.iter().any(|x| !x.is_finite()) {
                    bail!("--fault-matrix needs 9 finite numbers, row-major");
                }
                args.fault = Fault([[v[0], v[1], v[2]], [v[3], v[4], v[5]], [v[6], v[7], v[8]]]);
            }
            other => bail!("unknown argument {other}"),
        }
    }
    if args.out.is_empty() {
        bail!("--out <path without extension> is required");
    }
    Ok(args)
}

fn main() -> Result<()> {
    let args = parse_args()?;
    let mut config = SimConfig::default();
    // No walls: a field far larger than any episode can travel.
    config.field.length = 10_000.0;
    config.field.width = 10_000.0;
    config.validate().map_err(anyhow::Error::msg)?;

    let ticks_per_ep = ticks(args.seconds);
    let mut rows = Vec::with_capacity(args.episodes as usize * (ticks_per_ep + 1) * COLUMNS.len());
    let started = Instant::now();
    let mut steps = 0;
    for index in 0..args.episodes {
        steps += episode(&config, args.mode, args.fault, args.seed, index, ticks_per_ep, &mut rows);
    }
    let elapsed = started.elapsed().as_secs_f64();

    write_npy(&format!("{}.npy", args.out), &rows, COLUMNS.len())?;
    let meta = serde_json::json!({
        "columns": COLUMNS,
        "segments": SEG_NAMES,
        "mode": match args.mode { Mode::Mix => "mix", Mode::Waypoint => "waypoint" },
        "episodes": args.episodes,
        "seconds": args.seconds,
        "ticks_per_episode": ticks_per_ep,
        "tick": TICK,
        "seed": args.seed,
        "fault": { "name": args.fault_name, "matrix": args.fault.0 },
        "config": config,
        "sim_steps": steps,
        "sim_seconds": elapsed,
        "sim_us_per_step": elapsed * 1e6 / steps.max(1) as f64,
    });
    std::fs::write(format!("{}.json", args.out), serde_json::to_string_pretty(&meta)?)?;
    eprintln!(
        "{} episodes, {} rows, {} steps in {:.3} s ({:.2} us/step)",
        args.episodes,
        rows.len() / COLUMNS.len(),
        steps,
        elapsed,
        elapsed * 1e6 / steps.max(1) as f64
    );
    Ok(())
}
