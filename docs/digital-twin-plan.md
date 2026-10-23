# Plan: a deep-learning digital twin of the robot

Goal: a learned model that takes **the same commands the engine sends to the
real robot** and predicts how the real robot moves, so it can stand in for the
robot inside the engine (in place of, or next to, the analytic `ssl-sim`
physics) and Lua skills can be tuned against it with the existing headless
loop.

## 1. What the twin has to reproduce

### Inputs: the robot command, exactly as the robot receives it

The radio sends, per robot and per engine tick (60 Hz, `TICK` in
`src/main.rs`), the 5-byte packet built in `src/sender/packet_serializer.rs`:

| Field | Frame / range | Notes |
|---|---|---|
| `vx` | robot frame, forward, m/s | quantized to 0.01, clamped to ±5.11 |
| `vy` | robot frame, leftward, m/s | same |
| `omega` (`vth`) | rad/s, CCW | same |
| `kick` | 0/1 | `kick_x` |
| `dribbler` | 0–7 | 3 bits |

The twin's input is this **quantized, clamped** command (apply the same
`×100 → i32 → clamp` path), not the float Lua asked for: that is what the
firmware actually sees. This keeps the twin honest about saturation and
rounding, and it is also the `RobotCommand` (`vt`, `vn`, `omega`,
`kick_speed`) that `ssl-sim` already accepts, so the interface into the
simulator does not change.

### Outputs: the robot state the engine sees

`World` stores, per robot, `x, y, θ, vx, vy, ω` (`src/world.rs`,
`RobotState`). The twin predicts the next state of these from the current
state and the command history.

### What makes the real robot differ from `ssl-sim`

`Robot::drive` in `simulator/crates/sim/src/robot.rs` is a first-order model:
velocity tracks the command with a fixed acceleration cap, no delay, no
coupling. The real robot adds, and the model must learn:

1. **Latency**: radio + firmware + motor response + vision delay (typically
   60–150 ms, i.e. 4–9 ticks). Needs a command *history*, not just the
   current command.
2. **Non-ideal tracking**: per-axis gains, overshoot, different accel and
   decel, wheel slip at high acceleration, `vy` weaker than `vx` for an
   omni base, rotation coupling into translation (drift while spinning).
3. **Dead-zone** at low speeds (static friction, PWM dead band).
4. **Per-robot variation** and battery level (slower when the battery is low).
5. **Process noise**: the same command does not give the same motion twice.

Contacts (walls, robots, the ball) stay in the analytic simulator: the twin
replaces only the free motion of the robot body (see §5).

## 2. Data collection (the most important phase)

The model is only as good as the logs. Nothing in the engine logs
(state, command) pairs today, so this is the first piece of work.

### 2.1 Twin logger in the engine

Add `src/logger/twin_log.rs`, enabled by a toolbar/console toggle and a
`--twin-log <dir>` flag. Each engine tick it writes one row per active robot
to a CSV (one file per session, with a JSON sidecar holding robot id, team,
date, battery voltage at start/end, field, firmware version):

```
tick, t_engine, id, team,
cmd_vx, cmd_vy, cmd_omega, cmd_kick, cmd_dribbler,     # quantized, as serialized
raw_x, raw_y, raw_theta, t_capture, camera_id, seen,   # raw SSL-Vision detection
ekf_x, ekf_y, ekf_theta, ekf_vx, ekf_vy, ekf_omega     # tracker output (reference only)
```

- Hook point for the command: `Radio::send_commands` (`src/sender/radio.rs`),
  right where `serialize` is called, so the logged command is byte-for-byte
  what went over the air. Log the zero command for active robots too.
- Hook point for vision: `run_vision` (`src/receiver/vision.rs`), before the
  Kalman filter. **Train on raw detections, not EKF output**: the EKF uses a
  constant-velocity model and lags, and the twin would learn the filter's
  dynamics instead of the robot's. Keep `t_capture` so latency can be
  measured.
- Missing detections (`seen = 0`) are logged, not dropped, so the dataset
  keeps a fixed 60 Hz grid.

### 2.2 Excitation scripts (Lua, real robot)

Add `lua/twin/` with scripts that drive one robot through a field-safe
geofence (stop and return to center when within 0.5 m of a wall):

| Script | Purpose |
|---|---|
| `steps.lua` | step inputs per axis (vx, vy, ω) at 0.25 … 3.5 m/s and back to 0: latency, rise time, overshoot, decel |
| `chirp.lua` | sine sweeps 0.2–5 Hz per axis: bandwidth |
| `prbs.lua` | random piecewise-constant commands on all axes at once, held 0.1–1 s: coupling, broad coverage |
| `spin_translate.lua` | constant ω with constant vx/vy: rotation–translation coupling |
| `creep.lua` | slow commands (< 0.3 m/s, < 1 rad/s): dead-zone |
| existing skills/roles | `move_to`, `kick_to_point`, plays: the command distribution the twin will actually see |

### 2.3 Volume and splits

- Target ≥ 2 h of driving per robot (~430k ticks), across ≥ 3 battery levels.
- Split **by session**, never by random tick (neighbouring ticks leak):
  70 % train, 15 % validation, 15 % test; keep one whole session of
  gameplay-skill data as the final test.
- Record the same scripts in `ssl-sim` (headless) too: a sim dataset for
  pipeline debugging before real data is available.

## 3. Pre-processing

1. Unwrap θ; fill single-frame vision drops by interpolation, cut the
   sequence at longer gaps (> 3 frames); drop frames with camera hand-off
   jumps (> 5 cm in one tick).
2. Velocity targets: central differences of raw pose smoothed with a
   zero-phase Savitzky–Golay filter (offline, non-causal: allowed because it
   only builds targets).
3. **Express everything in the robot frame**: body velocities
   `(v_t, v_n, ω)` and pose deltas `(Δt, Δn, Δθ)` in the frame of the robot
   at time *k*. This makes the model invariant to field position and heading
   by construction (no need to learn it, no augmentation needed for it).
4. Estimate latency per robot by cross-correlating command and measured
   velocity; report it (it is also a useful hardware metric).
5. Normalize features with train-set statistics, saved with the model.

## 4. Model

### 4.1 Baselines first

1. **Current `ssl-sim` model**, as configured.
2. **Fitted `ssl-sim` model**: fit `max_accel`, `max_alpha`, `max_speed`,
   `max_omega` plus a pure delay of *d* ticks by least squares on the train
   set. Cheap, and it tells how much a DL model has to win.

The DL model ships only if it beats baseline 2 on the metrics in §6.

### 4.2 Recommended architecture: physics prior + learned residual (GRU)

```
inputs at tick k:
  state   s_k = (v_t, v_n, ω)                 body frame
  command u_{k-H..k} = (vx, vy, ω, kick, dribbler)   H = 12 ticks (200 ms)
  robot embedding e_id (8-d, learned)          per physical robot
  (optional) battery voltage

prior   = fitted first-order model with delay   (baseline 2)
GRU(64, 2 layers) over [s, u, e_id] sequence → MLP head
output  = (μ, log σ) of Δ(v_t, v_n, ω) relative to the prior
next velocity = prior + μ (+ σ·ε when sampling noise)
pose update   = integrate body velocity over the tick (same as Robot::drive)
```

Why this shape:

- **Residual on a physics prior**: the model stays sane outside the data
  (e.g. commands it never saw default to plausible motion), trains faster,
  needs less data.
- **GRU with command history**: captures latency and motor lag; the hidden
  state is cheap to carry per robot in the simulator.
- **Predict velocity, integrate pose**: integration is exact geometry, no
  need to learn it, and drift stays bounded by velocity error.
- **Gaussian head (NLL loss)**: gives the twin realistic noise, which makes
  skills tuned on it more robust; set σ = 0 for deterministic runs.
- **Robot embedding**: one model for the whole team, each robot keeps its
  quirks; a new robot needs only a few minutes of data to fit its embedding.

Alternatives to try in the same pipeline, in this order: MLP over a flattened
window (simplest, often enough); a small TCN (1-D dilated convolutions) if the
GRU is slow to train. Keep the model small (≤ 50k parameters): it must run for
16 robots at 60 Hz, faster than real time, on a laptop CPU.

### 4.3 Training

- Framework: Python + PyTorch, in a new top-level `twin/` directory
  (`twin/data.py`, `twin/models.py`, `twin/train.py`, `twin/eval.py`,
  `twin/export.py`, `pyproject.toml`).
- Loss: Gaussian NLL on one-step velocity deltas **plus multi-step rollout
  loss**: unroll the model on its own predictions for *N* ticks and penalize
  pose and velocity error along the way. Curriculum *N* = 1 → 10 → 30 → 60
  (1 s). One-step-only models look good on paper and drift in simulation;
  rollout loss is what makes a usable twin.
- Sequences of 2 s, batch 256, AdamW, lr 1e-3 cosine decay, early stopping
  on validation rollout error at 1 s.
- Augmentation: mirror y (flip `vy`, `ω`, `v_n`) to double the data and
  remove left/right bias; small noise on input states (robustness to its own
  errors in rollout).

## 5. Integration into the engine

### 5.1 Export

`twin/export.py` writes `models/robot_twin.onnx` plus
`models/robot_twin.json` (normalization stats, history length, latency, robot
embeddings by `(team, id)`, model version, training data hash).

### 5.2 Rust inference

Use [`tract-onnx`](https://crates.io/crates/tract-onnx): pure Rust, no system
libraries, matches the project's "no external tools" build. Inference is CPU,
deterministic for a given input, ~tens of µs per robot step for a model this
size.

### 5.3 Pluggable dynamics in `ssl-sim`

- Add a `Dynamics` trait in `simulator/crates/sim/src/robot.rs`:

  ```rust
  pub trait Dynamics: Send {
      /// New body velocity after `dt`, given the robot and its command.
      fn step(&mut self, robot: &Robot, dt: f64) -> (Vec2, f64);
  }
  ```

  `Robot::drive`'s current code becomes `AnalyticDynamics`, the default.
- The learned implementation lives **outside** `ssl-sim` (keep `ssl-sim`
  dependency-light) in `src/sender/simulator/twin.rs`, holding per-robot GRU
  hidden state and command history.
- The twin runs at the 60 Hz control tick, not at the 1 ms physics substep:
  it gives a target body velocity once per tick, and substeps integrate it
  and resolve walls/robot/ball contacts exactly as today. Contact handling
  stays analytic.
- Config in `simulator.toml`:

  ```toml
  [robot]
  dynamics = "learned"            # "analytic" (default) | "learned"
  model = "models/robot_twin.onnx"
  noise = false                   # sample σ (true) or use the mean (false)
  seed = 0                        # keeps runs deterministic when noise = true
  ```

- Works everywhere the simulator does: GUI `--sim`, `--headless` Lua tests,
  the `improve-skill` loop, MCP. Headless determinism is preserved (no noise,
  or seeded noise).
- Teleports reset the robot's hidden state and command history to "at rest".

### 5.4 Validation hooks in the GUI

- Charts panel: overlay real robot velocity vs. twin prediction live (run
  the twin in "shadow mode" on real commands while driving the real robot).
  The running error is a continuous health check of the twin and flags when
  a robot's hardware changed (worn wheels, new motor) and needs re-training.

## 6. Evaluation

On the held-out test sessions:

| Metric | What it shows | Target (initial) |
|---|---|---|
| one-step velocity RMSE (v_t, v_n, ω) | local accuracy | < baseline 2 by 30 % |
| open-loop rollout position error at 0.5 / 1 / 2 s | drift | < 5 / 10 / 25 cm median |
| open-loop heading error at 1 s | rotation | < 5° median |
| latency / rise time / overshoot on step tests | matches the hardware response | within 1 tick / 10 % / 10 % |
| NLL / calibration (fraction of real inside ±2σ) | noise realism | 90–98 % |
| **closed-loop sim-to-real gap** | the real goal | see below |

Closed-loop test: run the same Lua skill (`move_to` to a set of targets,
`kick_to_point` benchmark from `lua/tests/`) on the real robot and in the
headless engine with the twin. Compare time-to-target, overshoot, final
error, success rate. The twin succeeds when these differ from the real robot
less than they do under the analytic simulator.

## 7. Phase 2 (after the motion twin works)

- **Kicker**: learn actual kick speed and direction vs. command and
  approach speed, from logged `kick` events (vision ball velocity after the
  kick). Replaces `KICK_SPEED` in `src/sender/simulator/mod.rs`.
- **Dribbler**: ball-holding success and ball loss while turning/moving.
- **Vision noise and latency model**, so the engine's tracker sees
  twin-generated detections like real ones (tests the EKF too).
- **Battery** as an explicit input if a telemetry return channel is added
  (the radio's `callback` bit is already reserved).

## 8. Milestones

| # | Deliverable | Done when |
|---|---|---|
| M1 | Twin logger + `lua/twin/` excitation scripts | a session on a real robot gives a clean 60 Hz CSV; same scripts run headless |
| M2 | `twin/` data pipeline + baselines 1 and 2 | report with latency per robot and baseline metrics |
| M3 | GRU residual model, trained with rollout loss | beats baseline 2 on §6 metrics on test sessions |
| M4 | ONNX export + `Dynamics` trait + `tract` integration | `--headless` runs with `dynamics = "learned"`, deterministic, ≥ 20× real time with 16 robots |
| M5 | Closed-loop validation + GUI shadow mode | sim-to-real gap smaller than with analytic dynamics |
| M6 | Phase 2 (kick, dribbler, vision) | as needed |

## 9. Risks

- **Not enough varied data** → model extrapolates badly. Mitigations: the
  physics prior, excitation scripts that cover the command space, rollout
  error reported per command region.
- **Compounding error in long rollouts** → rollout loss, input-noise
  augmentation, predicting velocity rather than pose.
- **Vision artifacts** (camera hand-off, occlusion) learnt as dynamics →
  cleaning in §3, train on raw data with explicit `seen` flag.
- **Hardware drift** (battery, wear) → robot embedding, battery logging,
  shadow-mode monitoring, cheap re-fit of the embedding only.
- **Real-robot safety during data collection** → geofence in every
  excitation script, speed caps, a human on the stop button.
