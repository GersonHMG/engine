# Plan: a deep-learning digital twin of the robot

Goal: a learned model of the robot's **dynamics**: given the commands the
engine sends to the real robot, predict how the robot body moves. The model
is trained and evaluated in Python; exporting it and plugging it into the
engine's simulator come later and are not part of this plan.

The real robot data is noisy (vision jitter, dropped frames, camera
hand-offs). The model must learn the robot's deterministic response to
commands and **not** the noise, so noise handling runs through every step
below.

## 1. What the twin has to reproduce

### Input: the motion command, exactly as the robot receives it

The radio sends, per robot and per engine tick (60 Hz, `TICK` in
`src/main.rs`), the packet built in `src/sender/packet_serializer.rs`. The
model uses its motion part:

| Field | Frame / range | Notes |
|---|---|---|
| `vx` | robot frame, forward, m/s | quantized to 0.01, clamped to ±5.11 |
| `vy` | robot frame, leftward, m/s | same |
| `omega` (`vth`) | rad/s, CCW | same |

The input is the **quantized, clamped** command (the same `×100 → i32 →
clamp` path), not the float Lua asked for: that is what the firmware sees.
Kick and dribbler bits are logged but are not model inputs: they act on the
ball, not on the body's motion.

### Output: the body's motion

The model predicts the robot's body velocity `(v_t, v_n, ω)` (forward,
leftward, angular, in the robot frame) at the next tick. The pose
`(x, y, θ)` follows by integrating that velocity, which is exact geometry and
does not need to be learned.

### What the dynamics contain

`Robot::drive` in `simulator/crates/sim/src/robot.rs` is a first-order model:
velocity tracks the command with a fixed acceleration cap, no delay, no
coupling. The real robot's dynamics add:

1. **Latency**: radio + firmware + motor response + vision delay (to be
   measured; typically several ticks). Needs a command *history*.
2. **Non-ideal tracking**: per-axis gains, overshoot, different accel and
   decel, wheel slip at high acceleration, `vy` weaker than `vx`, rotation
   coupling into translation (drift while spinning).
3. **Dead-zone** at low speeds (static friction, PWM dead band).
4. **Per-robot differences** (wheels, motors).

Contacts (walls, other robots, the ball) are out of scope: data where the
robot touches anything is removed (§3).

## 2. Data collection

Nothing in the engine logs (state, command) pairs today, so this is the
first piece of work.

### 2.1 Twin logger in the engine

Add `src/logger/twin_log.rs`, enabled by a toolbar/console toggle and a
`--twin-log <dir>` flag. Each engine tick it writes one row per active robot
to a CSV (one file per session, with a JSON sidecar: robot id, team, date,
battery voltage at start and end, firmware version):

```
tick, t_engine, id, team,
cmd_vx, cmd_vy, cmd_omega, cmd_kick, cmd_dribbler,     # quantized, as serialized
raw_x, raw_y, raw_theta, t_capture, camera_id, seen,   # raw SSL-Vision detection
ekf_x, ekf_y, ekf_theta, ekf_vx, ekf_vy, ekf_omega     # online tracker, reference only
```

- Command hook: `Radio::send_commands` (`src/sender/radio.rs`), where
  `serialize` is called, so the logged command is byte-for-byte what went
  over the air. Log the zero command of active robots too.
- Vision hook: `run_vision` (`src/receiver/vision.rs`), **before** the Kalman
  filter. The engine's EKF is causal and uses a constant-velocity model, so
  its velocities lag the real ones; training on them would teach the model
  the filter's lag as if it were robot dynamics. Raw detections are denoised
  offline instead (§3), which has no lag.
- Missing detections (`seen = 0`) are logged, not dropped, so the dataset
  keeps a fixed 60 Hz grid.

### 2.2 Recordings

Add `lua/twin/` with scripts that drive one robot inside a geofence (stop and
return to the center when within 0.5 m of a wall):

| Script | Purpose |
|---|---|
| `still.lua` | robot at rest, zero command, 2 min, in each camera's area: **measures the vision noise** (§3.1) |
| `steps.lua` | step inputs per axis (vx, vy, ω) at 0.25 … 3.5 m/s and back to 0: latency, rise time, overshoot, decel |
| `chirp.lua` | sine sweeps 0.2–5 Hz per axis: bandwidth |
| `prbs.lua` | random piecewise-constant commands on all axes at once, held 0.1–1 s: coupling, broad coverage |
| `spin_translate.lua` | constant ω with constant vx/vy: rotation–translation coupling |
| `creep.lua` | slow commands (< 0.3 m/s, < 1 rad/s): dead-zone |
| `creep_long.lua` | slow single-axis holds of 1.5–4 s: steady-state friction at low speed |
| `spin_fast.lua` | 2–3 m/s while spinning 3–6 rad/s: wheel saturation |
| `sprint.lua` | line up at one end, run the full field at 1.5–3 m/s in a random body direction, brake: sustained high speed |
| existing skills | `move_to`, plays: the command distribution the model will meet |

`creep_long`, `sprint` and `spin_fast` were added after synthetic experiments
(`twin/`): with the other scripts, the geofence cut every fast run short and
slow steady driving was only ~2 s per robot, fast driving while spinning ~2 %
of the data, and the model was weakest exactly there. Check coverage (time spent per speed/acceleration region)
after every recording day.

Repeat each excitation script several times: repeated runs of the same
command sequence let the noise average out and show how much of the
variation is noise and how much is dynamics.

### 2.3 Volume and splits

- Target ≥ 2 h of driving per robot (~430k ticks).
- Split **by session**, never by random tick (neighbouring ticks leak):
  70 % train, 15 % validation, 15 % test; keep one whole session of
  skill driving as the final test.
- Record the same scripts in `ssl-sim` (headless) too, adding synthetic
  vision noise matching §3.1: a dataset with a known ground truth to check
  that the pipeline recovers the true dynamics from noisy data.

## 3. Pre-processing and noise handling

### 3.1 Characterize the noise

From `still.lua` recordings, per camera: standard deviation of `x`, `y`, `θ`,
whether it is white or correlated, and the outlier rate. This gives:

- the measurement noise for the smoother (§3.3);
- the **noise floor**: the error a perfect model would still show against
  the raw data. Evaluation (§6) is reported against this floor, so a model
  is never pushed to "explain" noise.

### 3.2 Clean

1. Unwrap θ.
2. Remove outliers: detections that jump more than physically possible in
   one tick (> 5 cm, > 0.3 rad), camera hand-off jumps, wrong-robot
   detections.
3. Gaps: fill single missing frames by interpolation and mark them; cut the
   sequence at longer gaps (> 3 frames).
4. Remove contacts: drop segments where the robot is within one radius of a
   wall, another robot or the ball, so collisions are not learned as
   dynamics.

### 3.3 Denoise offline (states for model input and targets)

Differentiating noisy positions amplifies the noise, so velocities are never
computed by plain finite differences. Instead run an offline
**Rauch–Tung–Striebel (RTS) smoother**: a forward Kalman filter plus a
backward pass over the whole sequence, using a constant-acceleration model
and the measurement noise from §3.1. Being non-causal it has no lag, unlike
the engine's online EKF. Output: smoothed pose, body velocity and their
variances per tick. (Savitzky–Golay zero-phase filtering is a simpler
fallback; compare both on the `ssl-sim` dataset, where the truth is known.)

The smoother must not impose dynamics of its own: keep its process noise
loose (it only removes measurement noise), and check on the `ssl-sim`
dataset that smoothed velocities match the true ones without visible lag or
rounding of step responses.

### 3.4 Features

1. **Robot frame**: body velocities `(v_t, v_n, ω)` and pose deltas in the
   frame of the robot at tick *k*. Invariant to field position and heading by
   construction.
2. Estimate latency per robot by cross-correlating command and smoothed
   velocity; report it.
3. Normalize with train-set statistics, saved with the model.

## 4. Model

### 4.1 Baselines first

1. **Current `ssl-sim` model**, as configured.
2. **Fitted `ssl-sim` model**: fit `max_accel`, `max_alpha`, `max_speed`,
   `max_omega` plus a pure delay of *d* ticks by least squares on the train
   set.
3. **Linear model**: ARX (linear regression on past velocities and
   commands). Shows how much of the dynamics is linear.

The deep model is worth keeping only if it beats baseline 2 by a clear
margin on the §6 metrics.

### 4.2 Recommended architecture: physics prior + learned residual (GRU)

```
inputs at tick k:
  state   s_k = (v_t, v_n, ω)                     smoothed, body frame
  command u_{k-H..k} = (vx, vy, ω)                H = 12 ticks (200 ms)
  robot embedding e_id (8-d, learned)             per physical robot

prior   = fitted first-order model with delay       (baseline 2)
GRU(96, 2 layers) over [s, u_{k-H..k}, e_id] → MLP head
output  = Δ(v_t, v_n, ω) relative to the prior      deterministic
next velocity = prior + Δ
pose update   = integrate body velocity over the tick
```

- **Deterministic output**: the model predicts the expected motion only.
  Noise is a property of the sensor, not of the dynamics, so there is no
  noise head and no sampling.
- **Residual on a physics prior**: plausible behaviour outside the data,
  faster training, less data needed.
- **GRU with command history**: captures latency and motor lag.
- **Small** (~100k parameters): with noisy targets, a large model overfits
  the noise first. Feed the whole command window to the GRU at every tick,
  not only the current command: on the synthetic robots, that change
  (together with 96 units and longer training) cut the 1-s error by ~25 %
  and most of a spin-rate bias while translating.

Alternatives in the same pipeline: MLP over a flattened window (simplest);
a small TCN if the GRU trains slowly.

### 4.3 Training (noise-aware)

- Python + PyTorch in `twin/` (implemented: see `twin/README.md`).
- **The model runs in a loop on its own predictions** (as a simulator, and
  inside the engine's control loop), so it is trained that way: every
  training sample warms the GRU up on measured history, then free-runs it on
  its own outputs.
- **Multi-step rollout loss on raw measurements**: unroll the model on its
  own predictions for *N* ticks from a smoothed initial state, integrate the
  pose, and compare to the raw (cleaned) vision poses along the way. Vision
  noise is zero-mean and independent between frames, so it averages out
  over the horizon instead of being fitted; it is also what makes the model
  stable in long simulations. Curriculum *N* = 10 → 30 → 60 → 120 (2 s);
  stopping at 1 s left visible errors in 2-s maneuvers and closed-loop runs.
- Add a one-step velocity term against the smoothed velocities, weighted by
  the smoother's inverse variance, so confident samples count more.
- **Huber loss** on both terms, so remaining outliers do not dominate.
- **Input noise injection**: add noise of the §3.1 size to input states
  during training, so the model does not rely on exact state values and
  stays stable on its own predictions.
- Regularization: weight decay, dropout in the head, early stopping on
  validation rollout error at 2 s.
- Delay: chosen per robot by fitting the prior with each candidate delay and
  keeping the best on validation (cross-correlation alone overestimated it).
- **No mirror augmentation**: flipping y assumes a left/right symmetric robot,
  which is exactly what a faulty robot (bad wheel) is not.
- **Recovery training**: in a quarter of the windows, knock the start
  velocity off and score only the velocity after a 0.25 s grace period. A
  model only ever started from correct states can lock into a wrong state
  once it is fed its own predictions (seen in a 2-min free run).
- Validate the whole pipeline first on the noisy `ssl-sim` dataset: the
  trained model must recover the simulator's true (noise-free) response.

## 5. Using the model as a simulator (Python)

No export yet: the model stays a PyTorch checkpoint
(`twin/checkpoints/<name>.pt`, with normalization stats, history length,
latency and robot embeddings) and is used from Python:

```python
twin = Twin.load("twin/checkpoints/v1.pt", robot=(0, 0))  # (id, team)
twin.reset(x=0.0, y=0.0, theta=0.0)                       # at rest
for cmd in commands:                                     # (vx, vy, omega), robot frame
    state = twin.step(cmd)                               # x, y, θ, v_t, v_n, ω after 1/60 s
traj = twin.rollout(initial_state, commands)             # whole sequence at once
```

- `step` keeps the GRU hidden state and the command history; `reset`
  puts the robot at rest.
- Commands go through the same quantize-and-clamp as `packet_serializer.rs`.
- Deterministic: the same inputs always give the same trajectory.
- `twin/replay.py` writes rollouts in the engine's replay CSV format, so
  real and predicted trajectories can be compared visually in the engine's
  replay panel.

## 6. Evaluation

On the held-out test sessions, by replaying the logged command sequences
through the model open-loop and comparing to what the real robot did:

| Metric | What it shows |
|---|---|
| one-step velocity RMSE (v_t, v_n, ω) vs. smoothed velocity | local accuracy |
| open-loop rollout position error at 0.5 / 1 / 2 s vs. raw vision | drift |
| open-loop heading error at 1 s | rotation |
| latency, rise time, overshoot, settling on step tests | match to the hardware response |
| error per command region (speed, accel, low-speed) | where the model is weak |
| **free run of whole sessions** (minutes) on its own predictions | stability in the loop: velocity error must not grow |
| **closed loop**: a controller drives the twin from the twin's own pose vs. the same controller on the robot | the real use; settle time and path separation |

Noise-aware reading of the results:

- **Compare against the noise floor** (§3.1). A one-step error close to the
  vision noise is as good as possible; going below it means the model is
  fitting noise.
- **Residual whiteness**: the model's one-step residuals should look like the
  measured vision noise: no autocorrelation, no correlation with the
  command. Leftover structure means dynamics not yet captured.
- **Repeated runs**: on the repeated excitation runs (§2.2), compare the
  prediction with the average of the repeats, which has much less noise
  than any single run.
- **Ground-truth check**: on the noisy `ssl-sim` dataset, error against the
  noise-free truth, which shows directly whether the dynamics were
  recovered.

Targets are set once the baseline numbers and the noise floor are known: the
model should beat baseline 2 clearly on rollout error at 1 s and be close to
the noise floor on one-step error.
