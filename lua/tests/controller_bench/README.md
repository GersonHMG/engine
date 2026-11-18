# Controller benchmark

Runs one robot controller, written in Lua, on three robot profiles × three tests in the headless
simulator. It prints the metrics and compares them with a recorded baseline. The baseline
controller is a plain bang-bang (`controllers/bang_bang.lua`). Better controllers are measured
against it.

```bash
./target/debug/engine.exe --headless --max-time 600 lua/tests/controller_bench/run.lua
CB_CONDITION=ideal ./target/debug/engine.exe --headless --max-time 600 lua/tests/controller_bench/run.lua
```

A run takes under a second of wall time (about 110 s of simulated time). The simulator is
deterministic: vision noise uses a fixed seed, and two runs give the same numbers. One run is
enough, and any change comes from what you changed.

| Variable | Default | |
|---|---|---|
| `CB_CONTROLLER` | `bang_bang` | `controllers/<name>.lua` |
| `CB_CONDITION` | `realistic` | `ideal` (no noise, no delay) or `realistic` (1.5 mm / 0.002 rad vision noise, about SSL-Vision's jitter (an estimate, not measured on our cameras); 33 ms vision + 33 ms radio delay) |
| `CB_POSITION_NOISE`, `CB_ANGLE_NOISE`, `CB_VISION_DELAY`, `CB_RADIO_DELAY` | — | override one value of the condition (m, rad, s, s); it shows in the condition's name, and there is no baseline to compare with |
| `CB_ROBOTS` | `perfect_robot,one_bad_wheel,bad_torque` | profiles in `robots/<name>.toml` |
| `CB_TESTS` | `go_to_point,square,kick_to_point` | |
| `CB_TRACE` | — | a folder: each test's trajectory is written there as `<test>_<robot>.csv`: the true state (`x, y, theta, vx, vy`), the pose the controller was told to reach that tick (`goal_x, goal_y, goal_theta`) and the ball as vision sees it (`ball_x, ball_y`) |

During the run:

- Only blue 0 is on the field. The other robots are removed and put back at the end.
- The engine's command ramp is turned off (`motion all ramp_accel 0`), so the controller owns
  the acceleration.
- The robot profile, vision, radio and ramp settings are put back at the end.

## Robots

All three robots have `max_accel` 6 m/s², `max_speed` 3.5 m/s and `grip_accel` 8 m/s².

| Profile | Wheels | What it can do |
|---|---|---|
| `perfect_robot` | no torque limit | 6 m/s² in every direction |
| `one_bad_wheel` | 0.05 N·m ×3, front-right 0.03 N·m | up to about 3.4 m/s², less in directions that load the weak wheel |
| `bad_torque` | 0.03 N·m ×4 | about 2.1 m/s² forward, 2.5 m/s² sideways: below what the controller asks for (2.5 m/s²) |

## Tests

Every test starts with blue 0 at rest at (0, 0), facing +x.

| Test | What |
|---|---|
| `go_to_point` | Go to (2.0, 1.0) and turn to 90°. Runs 6 s. |
| `square` | Drive a 1.5 m square, (0,0) → (1.5,0) → (1.5,1.5) → (0,1.5) → (0,0), facing +x. A corner counts once the robot is within 5 cm of it and slower than 0.3 m/s; then the next corner is the target. Ends 1 s after the last corner, or at 20 s. |
| `kick_to_point` | Ball at (1.0, 0.5), target (3.0, −1.0). The robot goes 0.2 m behind the ball on the kick line, facing the target. It pushes through the ball with the kicker armed once it is lined up: within 2 cm of the line, within 0.03 rad, slower than 0.1 m/s. If it hasn't lined up after 5 s it pushes anyway, and `align_s` shows `NO`. The kick speed is the profile's `kick_x_speed` (3 m/s). The ball is followed for 2.5 s after the kick. Ends at 12 s. |

The tests decide phases (corner reached, lined up) from the robot's true state. The
controller only sees vision.

## Metrics

All are measured on the robot's **true** state (`sim.truth`), not on vision. Lower is better,
except `kicked` and `corners`.

**go_to_point**

| Column | Meaning |
|---|---|
| `reach_s` | **primary.** First time within 5 cm and 0.1 rad of the pose. `NO` = never. |
| `settle_s` | Earliest time after which it stays within 2 cm and 0.05 rad until the end (at least the last 0.25 s). `NO` = it never stays still on the pose, for example if it keeps hunting around it. |
| `plan_s` | Reference: rest-to-rest time with the controller's own limits (`v_max`, `a_max`, …). The time the robot would take if it did exactly what it is told. |
| `os_mm` | Overshoot past the target, along the start→target line. |
| `dev_mm` | Largest distance from the straight start→target line. |
| `hold_mm`, `hold_deg` | RMS position and heading error over the last 1 s: how still it holds the pose. |
| `v_peak` | Peak speed reached. |

**square**

| Column | Meaning |
|---|---|
| `lap_s` | **primary.** Time when the last corner is reached. `NO` = it didn't finish in 20 s. |
| `plan_s` | Reference: 4 × the rest-to-rest time of one 1.5 m edge. |
| `corners` | Corners reached (of 4). |
| `rms_mm`, `max_mm` | RMS and largest distance from the square's outline. |
| `os_mm` | Largest overshoot past a corner, along the edge arriving at it. |
| `head_deg` | Largest heading error. The robot should keep facing +x. |
| `hold_mm` | RMS distance from (0,0) over the last 1 s. Only when the lap was finished. |

**kick_to_point**

| Column | Meaning |
|---|---|
| `miss_mm` | **primary.** Closest the ball passes to the target. `-` = no kick. |
| `kicked` | 1 if the kick fired. |
| `align_s` | Time when it was lined up. `NO` = it gave up and pushed anyway. |
| `kick_s` | Time of the kick. |
| `dir_deg` | Error in the ball's direction (kick point → ball 0.3 s later) against the kick line. |
| `ball_v` | Ball's mean speed over those 0.3 s. |

### Score

Every test also has a `cost`, in seconds or a seconds equivalent. Lower is better:

| Test | cost |
|---|---|
| `go_to_point` | `reach_s` (10 if never) + `hold_mm` / 50 (at most 10) |
| `square` | `lap_s`; if unfinished, 20 + 5 per corner not reached |
| `kick_to_point` | `kick_s` / 2 + `miss_mm` / 100 (at most 10); 15 without a kick |

`SCORE` is the sum of the costs of every robot × test, per condition. `track.py` logs it as
`score` per child run, `<condition>.score` and `score.total` (ideal + realistic) on the
parent run. `score.total` is what the `improve-controller` skill minimizes.

The bang-bang baseline scores ideal 36.29, realistic 152.57, total 188.87. The noise
sequence (`SIM_SEED=1`, `2`) moves the realistic score by about 4%.

Each run writes `results/latest_<condition>.csv` (ignored by git). When
`results/baseline_<condition>.csv` exists, it prints every number's change (new − baseline).

### Improving it automatically

In a new Claude Code session, run `/improve-controller [name] [idea]`. It copies the current
best controller (the one in `results/baseline_realistic.csv`) to `controllers/<name>.lua`
and improves it one hypothesis at a time, logging every attempt to MLflow. It works on a
`controller/<name>` branch and writes a report in `reports/`. The rules are in
`.claude/skills/improve-controller/SKILL.md`.

`track.py` logs into the main checkout's store, also from a worktree, so every session's
runs end up in one MLflow experiment.

## Baseline: bang-bang (2026-11-18, realistic noise 1.5 mm / 0.002 rad)

The controller sends raw velocities, planning with `v_max` 2.5 m/s, `a_max` 2.5 m/s²,
`omega_max` 6 rad/s, `alpha_max` 20 rad/s². It uses the same limits for every robot.

The target speed along the error is `min(v_max, sqrt(2·a_max·d))`. The command moves toward it
by at most `a_max·dt` per tick. The heading works the same way. The command ramps from the
controller's last command, not from the measured velocity.

```
ideal          go_to_point:reach_s  square:lap_s  kick_to_point:miss_mm
perfect_robot                 1.68          5.82                     42
one_bad_wheel                 2.23          7.80                     16
bad_torque                    2.37          8.90                     51

realistic
perfect_robot                 2.55          8.72                    327
one_bad_wheel                   NO            NO           - (no kick)
bad_torque                      NO            NO           - (no kick)
```

The full tables are in `results/baseline_ideal.csv` and `results/baseline_realistic.csv`.

What it shows:

- **Without delay it works on every robot, but plans past what the weak ones can do.**
  `bad_torque` can't reach 2.5 m/s². Its command runs ahead of its real speed, so it brakes too
  late: 710 mm overshoot on `go_to_point` against 27 mm for the perfect robot. It doesn't settle in 6 s;
  the perfect robot settles in 2.0 s. `one_bad_wheel` drifts sideways, deviating 374 mm from the straight line.
- **With 66 ms of round-trip delay it never settles, even on the perfect robot.** Near the
  target the `sqrt` braking curve has a very high gain, so the delayed state makes it hunt
  around the pose. On the perfect robot that is RMS 21 mm / 7.3° over the last second, so it never
  stays within 2 cm. It also turns the field→robot frame with a heading 66 ms old while
  spinning at up to 6 rad/s, which bends its path.
- **The weak robots go unstable with delay.** They hunt 0.5–0.7 m around the target and never
  finish the square or line up for the kick, and their pushes never reach the ball.
- **Kicking needs a still heading.** With delay the perfect robot never lines up within
  0.03 rad; it pushes anyway at 5 s, kicks 7.7° off and misses by 327 mm, against 42 mm
  without delay.
- **The delay is the problem, not the noise.** With the delays and no noise at all the results
  are about the same (perfect robot: lap 8.67 s, miss 348 mm; weak robots never finish). With
  noise and no delay they are close to `ideal` for the perfect robot. The heading hunting
  (about 7° RMS) is the same with angle noise 0.005, 0.002 or 0 rad.

Where a better controller can gain: latency compensation (predict the state 66 ms ahead), a
linear zone near the target instead of the pure `sqrt`, ramping from the measured velocity, and
limits per robot (or measured online) instead of one fixed `a_max`.

## Logging experiments to MLflow

`track.py` runs the benchmark, in both conditions by default, and logs it to a local MLflow
store in `mlflow/` (ignored by git). It needs [uv](https://docs.astral.sh/uv/); the first run
installs MLflow into uv's cache.

```bash
uv run lua/tests/controller_bench/track.py --note "bang-bang baseline"
uv run lua/tests/controller_bench/track.py --controller my_ctrl --note "latency compensation 66 ms"
uv run lua/tests/controller_bench/track.py --conditions realistic --set CB_ANGLE_NOISE=0 --note "no heading noise"
```

| Option | |
|---|---|
| `--controller` | `controllers/<name>.lua` (default `bang_bang`) |
| `--conditions` | `ideal,realistic` (default both) |
| `--robots`, `--tests` | as `CB_ROBOTS`, `CB_TESTS` |
| `--set CB_X=V` | any `CB_*` override (repeatable) |
| `--note` | what the experiment tries; it becomes the run's name |
| `--build` | `cargo build` first |

One invocation is one **parent run**, named by `--note`. Under it is one **child run** per
condition × robot. A child run has:

- **params:** `controller`, `condition`, `robot`, `controller.*` (v_max, a_max, …),
  `sim.vision.*` and `sim.radio.*`, and any `override.*`
- **metrics:** every column as `<test>.<metric>` (e.g. `square.lap_s`,
  `kick_to_point.miss_mm`), plus `<test>.ok` = 1 when the test's primary metric exists. A `NO`
  is not logged, so filter or sort with `<test>.ok`.
- **artifacts:** the trajectories (`trajectories/<test>_<robot>.csv`) and their plots
  (`plots/<test>_<robot>.png`, see below)

The parent run has every child's primary metric as `<condition>.<robot>.<test>.<metric>`, and
an overlay of all robots per test (`plots/<condition>/<test>_all.png`). Its other
artifacts are the controller's source, the robot profiles, the result CSVs, the engine's
output, and `git/uncommitted.diff` when tracked files had changes. Its tags are the git commit
and whether the tree had changes, so a run can be reproduced.

To open the UI:

```bash
uvx --python 3.12 --from "mlflow>=3,<4" mlflow ui --backend-store-uri sqlite:///lua/tests/controller_bench/mlflow/mlflow.db
```

Open http://127.0.0.1:5000. It starts in the **GenAI** view; switch to **Model training** to
see the runs table. To compare controllers on one robot, filter the child runs with
`params.robot = 'bad_torque' and params.condition = 'realistic'`, select them and press
**Compare**.

## Plots: target vs real

`plot.py` draws the traces. `track.py` attaches the plots to the runs, and you can also run it
on any `CB_TRACE` folder:

```bash
uv run lua/tests/controller_bench/plot.py <trace_dir> [<out_dir>]
```

One figure per test and robot:

- **Field (left):** the target path, the real path, and the ball. The target path is the
  setpoints the controller was given, joined by dashed straight lines from the start. The
  setpoints are marked × with an arrow for their heading. The real path has the robot's heading
  every 0.25 s. For the kick, it shows the ball as vision sees it and the kick line it should
  have followed.
- **Against time (right):** distance to the current setpoint (symlog, the dotted line is
  2 cm), the real heading vs the target heading, and speed.

`<test>_all.png` puts every robot's real path on one field, over the target path.

## Writing a controller

Copy `controllers/bang_bang.lua` to `controllers/<name>.lua`. A controller module has
`new(params)`, and the object it returns has:

- `params` with `v_max`, `a_max`, `omega_max`, `alpha_max` (used for `plan_s`)
- `:step(id, team, target)`, called once per tick with `target = {x, y, theta}`, which sends
  that tick's command
- `:reset()`, called when the robot is put back at the start

Run it with `CB_CONTROLLER=<name>` in both conditions and read the `change vs` section (or log
it with `track.py --controller <name>`). When it is better, copy `results/latest_<condition>.csv` to `results/baseline_<condition>.csv`.

The run needs the three profiles in `robots/`.
