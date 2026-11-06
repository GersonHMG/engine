# Motion-control benchmark

A fixed set of moves, run headless in the built-in simulator, scored on the robot's
**true** motion (no vision noise, no delay). It is the baseline to improve robot control
from: change the controller, rerun, and read the change against `results/baseline.csv`.

```bash
./target/debug/engine.exe --headless --max-time 600 lua/tests/motion_control/run.lua
```

A run takes about 3 s (170 s of simulated time). It prints a table per condition, one
`SUMMARY` line per condition, writes `results/latest.csv` and, when
`results/baseline.csv` exists, the change of every case and of the summary.

| Variable        | Default            | Meaning                                        |
|-----------------|--------------------|------------------------------------------------|
| `MC_CONTROLLER` | `baseline`         | `controllers/<name>.lua` to test               |
| `MC_CONDITIONS` | `ideal,realistic`  | which conditions to run, comma separated       |

```bash
MC_CONTROLLER=my_controller ./target/debug/engine.exe --headless --max-time 600 lua/tests/motion_control/run.lua
```

## Files

| File | |
|---|---|
| `run.lua` | entry point: runs the cases, measures, prints, writes the CSV, compares |
| `cases.lua` | the moves (keep it fixed; new cases go at the end, then make a new baseline) |
| `controllers/baseline.lua` | the controller the skills use today: `move_to` + `face_to` |
| `results/baseline.csv` | the reference run (committed) |
| `results/latest.csv` | the last run (ignored by git) |

## Cases

All with blue robot 0 starting at rest; the robot keeps facing +x unless a heading is given.

| Case | Move |
|---|---|
| `fwd_0.1m` … `fwd_4m` | straight ahead: 0.1, 0.5, 1, 2, 4 m |
| `back_1m`, `side_1m`, `diag_2m` | backwards, sideways, diagonal (robot keeps its heading) |
| `turn_90`, `turn_180` | turn in place |
| `fwd_2m_turn` | drive 2 m while turning 180° |
| `stop_moving` | heading for a far point; after 1.2 s the target jumps to a point just ahead |
| `reverse` | after 1.0 s the target jumps behind the moving robot |
| `track_line`, `track_circle` | follow a target moving at 0.8 m/s on a line / a 1 m circle |

Before each case the robot is held at its start for 30 ticks with no commands, so delayed
commands and vision frames from the previous case are flushed.

## Metrics

Point cases (reach and stop on a point, facing the heading):

| Column | Meaning |
|---|---|
| `settle_s` | earliest time after which the robot stays within **3 cm and 0.05 rad** of the target until the end (`NO` = never). For `stop_moving`/`reverse`, counted from the target jump. |
| `opt_s` | lower bound: rest-to-rest trapezoid time with the simulator robot's `max_speed`/`max_accel` (and `max_omega`/`max_alpha` for turns) |
| `ratio` | `settle_s / opt_s` (can be slightly < 1: settling only needs to be within 3 cm) |
| `os_mm`, `os_deg` | overshoot past the target along the move, and past the heading |
| `dev_mm` | largest distance from the straight start→target line |
| `err_mm`, `err_deg` | final error, mean of the last 0.25 s |
| `v_max`, `a_max` | peak speed and acceleration actually reached |

Tracking cases: `track_mm` = RMS / max distance to the moving target, after the first 1 s.

`SUMMARY` per condition:

- `settled` — point cases that settled.
- **`SCORE`** — total settling time of the point cases, with a case that never settles
  counted as its whole duration. Lower is better; this is the main number.
- `mean_ratio`, `mean_overshoot`, `mean_final_err`, `track_rms` — averages over the cases
  where they apply.

Some overshoot is physics, not the controller: in `stop_moving` the robot is at about
2.4 m/s when the target jumps 0.56 m ahead, and at 2 m/s² it needs about 1.4 m to stop.

## Conditions

| Condition | Vision noise | Vision delay | Radio delay |
|---|---|---|---|
| `ideal` | 0 | 0 | 0 |
| `realistic` | 5 mm position, 0.01 rad heading | 33 ms | 33 ms |

The `realistic` values are **guesses** until they are measured on the real robots (record
step responses, fit the delay and limits, put them in `simulator.toml` and here). The
robot itself (max speed, acceleration, …) is whatever `simulator.toml` says. The noise uses
a fixed seed, so a run repeats exactly: any change in the results comes from the change
you made.

## Improving a controller

1. Copy `controllers/baseline.lua` to `controllers/<name>.lua`. A controller is a
   `function(id, team, target, heading)` called once per tick that sends that tick's
   command (`send_velocity`, `move_to`, `face_to`, …).
2. Run with `MC_CONTROLLER=<name>` and read the `change vs results/baseline.csv` section
   (negative = better).
3. When a change is better overall (realistic `SCORE` first, without making overshoot,
   final error or tracking worse), make it the new reference:
   `cp lua/tests/motion_control/results/latest.csv lua/tests/motion_control/results/baseline.csv`.
4. For changes in the engine (Rust: `src/motion`), rebuild and run with the baseline
   controller.

The benchmark also needs the bang-bang limits it was run with to mean anything: the
current baseline was recorded with the engine defaults (max acceleration 2.5 m/s², max
speed 5 m/s, no `robot_settings.toml`).

## Baseline (2026-11-06, controller `baseline`)

```
SUMMARY ideal     settled=13/13 SCORE=22.17 s  mean_ratio=1.04  mean_overshoot=131.1 mm  mean_final_err=0.1 mm  track_rms=194.6 mm
SUMMARY realistic settled=12/13 SCORE=41.48 s  mean_ratio=2.07  mean_overshoot=309.5 mm  mean_final_err=18.7 mm  track_rms=205.3 mm
```

What it shows:

- **Delay and noise double the time to settle** (ratio 1.04 → 2.07) and add ~20 cm of
  overshoot on straight moves (e.g. `fwd_1m`: 1 mm → 204 mm). The controller does not
  account for latency at all.
- **Diagonal moves overshoot even in ideal conditions** (`diag_2m`: 249 mm, ratio 1.5),
  while straight moves don't.
- **Tracking lags by ~20 cm** at 0.8 m/s: `move_to` aims at where the target is now and
  plans to stop there, with no feed-forward of the target's velocity.
- `reverse` never settles with delay and noise (final error 70 mm).

## Known issues in the current control code

- `Motion::default()` plans with `BangBangControl::new(2.5, 5.0)` (max acceleration
  2.5 m/s²), while `simulator.toml` gives the robot `max_accel = 2.0`: the plan asks for
  more than the robot can do, and the difference shows up as overshoot.
- `face_to` creates a new PID (`Motion::new()`) on every call, so the I and D terms have
  no memory: the integral is one tick's worth and the "derivative" is
  `kd · error / dt` = 0.1 · 60 · error = 6 × error at 60 Hz, i.e. extra proportional gain.
