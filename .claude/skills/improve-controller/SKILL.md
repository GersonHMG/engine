---
name: improve-controller
description: Improve the robot motion controller (lua/tests/controller_bench/controllers/) by measuring it with the controller benchmark and logging every attempt to MLflow. Use when asked to improve, tune or design a better robot controller, or to beat the bang-bang baseline.
argument-hint: "[controller name] [idea to try first]"
---

You improve a Lua robot controller in a measure → change → measure loop, on your own, until
a stop condition. Do not ask the user to run anything; run every test yourself. The user
reviews the results afterwards in MLflow and in your report.

Read `lua/tests/controller_bench/README.md` first. It defines the robots, tests, metrics,
conditions and the controller API. This skill only adds the loop and its rules.

# Setup

1. **Check that the benchmark is there.** You need `lua/tests/controller_bench/` (`run.lua`,
   `track.py`, `plot.py`, `controllers/`, `results/baseline_*.csv`) and
   `robots/perfect_robot.toml`, `robots/one_bad_wheel.toml`, `robots/bad_torque.toml`. If any
   is missing (for example, a worktree made before they were committed), stop and tell the
   user to commit them.
2. **Engine.** Use `./target/debug/engine.exe`. In a git worktree without `target/`, use the
   main checkout's: `$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")/target/debug/engine.exe`.
   `track.py` finds it by itself. Build (`cargo build`) only if neither exists. You never
   change engine code, so you never need to rebuild.
3. **Tools.** `uv` must be on the PATH (`uv --version`). `track.py` installs MLflow and
   matplotlib through it on the first run.
4. **Branch.** If you are on `master` or `main`, create and switch to
   `controller/<name>`. Kept changes are committed there, never pushed.
5. **Name and starting point.**
   - The name is the skill's first argument, else `ctrl_<YYYYMMDD>`.
   - Start from the current best controller: the `controller` column of
     `results/baseline_realistic.csv`. Copy `controllers/<best>.lua` to
     `controllers/<name>.lua`, unless `<name>.lua` already exists; then continue from it.
   - Never edit `bang_bang.lua`, or any controller other than `<name>.lua`.
   - If the user gave an idea with the skill, it is your first hypothesis.
6. **Baseline run.** Log the starting point:
   ```bash
   uv run lua/tests/controller_bench/track.py --controller <name> --note "<name> start (copy of <best>)"
   ```
   The last lines are `SCORE ideal …`, `SCORE realistic …` and `SCORE total …`. The total is
   the number to beat. Write down the three scores and the MLflow run id.

# Rules

**Scope**
- Edit only `lua/tests/controller_bench/controllers/<name>.lua`.
- Do not touch:
  - `run.lua`, `track.py`, `plot.py`, `results/*.csv` (until the end) and the README
  - `robots/`, `scenarios/`, `simulator.toml`, `robot_settings.toml`
  - engine code (`src/`, `simulator/`)

  Changing any of these changes the exam, not the controller.

**Controller API.** The module returns `{ new = function(params) ... }`. The object has:
- `params` with `v_max`, `a_max`, `omega_max`, `alpha_max`, `dt`. They are the limits it
  plans with, and `plan_s` uses them.
- `:step(id, team, target)`, called once per tick with `target = {x, y, theta}`, which sends
  exactly one `send_velocity(id, team, vx, vy, omega)`: robot frame, m/s and rad/s.
- `:reset()`, which forgets per-robot state.

**What it may read**
- Allowed: `get_robot_state(id, team)` (vision: noisy and delayed), `math`, and its own
  state, keyed by team and id.
- Forbidden:
  - `sim.*` (`sim.truth` is the ground truth the tests score with) and `grsim.*`
  - `get_ball_state`
  - `move_to`, `move_direct`, `face_to` (the engine's own controllers)
  - `io`, `os`, `require` of anything outside `math`, and the robot profile files or console
    output: the controller doesn't know the robot.

**No test-fitting**
- Nothing in the controller may depend on which test, robot or condition is running: no
  hard-coded coordinates, names, run times or tick counts.
- It may estimate the robot online, for example measure the acceleration it actually gets.
  That has to work on any robot.
- Delays may be assumed only as one tunable constant (e.g. `LATENCY = 0.066` s), stated as an
  assumption in the header.

**Code**
- Lua 5.4. Tunable values are `UPPER_CASE` local constants at the top, with units.
- Keep the header comment up to date with what the controller does.
- Per tick it must stay cheap: the `perf: lua process() avg` line at the end of a run should
  stay well under 1 ms.

# Loop

Each attempt is one hypothesis and one change.

1. **Diagnose.** Find where the score is lost. The costs are on the `cost` column of each
   test's table. For the full tables:
   ```bash
   CB_CONTROLLER=<name> CB_CONDITION=realistic <engine> --headless --max-time 600 lua/tests/controller_bench/run.lua
   ```
   Add `CB_TRACE=<scratch dir>` for the trajectories (`t, x, y, theta, vx, vy, goal_*, ball_*`)
   and analyse them numerically: lag behind the setpoint, overshoot, oscillation period and
   amplitude, the heading error while driving, and commanded vs reached acceleration. Look at
   the robot that loses the most first. The plots in MLflow are for the user; work from the
   numbers.
2. **Hypothesis.** Write ONE hypothesis: what is wrong, what you change, and which robot ×
   test costs you expect to drop, roughly by how much.
3. **Change** `controllers/<name>.lua` only.
4. **Measure and log**, with the hypothesis as the note:
   ```bash
   uv run lua/tests/controller_bench/track.py --controller <name> --note "<name> #<n>: <hypothesis in a few words>"
   ```
   A Lua error or a failed run counts as a failed attempt: fix it or revert.
5. **Decide.** Compare with the best so far.
   - **Keep** if `SCORE total` is at least 3% lower, `SCORE ideal` is no more than 5% worse,
     and no test that passed (`<test>.ok` = 1, i.e. not `NO`) now fails.
   - **Gain under 3%:** confirm it on two more noise sequences:
     ```bash
     uv run lua/tests/controller_bench/track.py --controller <name> --conditions realistic --set SIM_SEED=1 --note "<name> #<n> seed 1"
     ```
     Repeat with `SIM_SEED=2`, and run the best version the same way if you don't have its
     numbers. Keep it only if the mean realistic score over the three sequences (default, 1
     and 2) is lower. Noise alone moves it by about 4%.
   - **Otherwise reject:** restore the file to the last kept version
     (`git checkout -- <file>`, or your copy).
6. **Commit** a kept change:
   `controller <name>: <change> (score <old> -> <new>, MLflow <run id>)`. Keep a list of every
   attempt: number, hypothesis, scores, run id, kept or rejected.
7. Repeat from 1.

**Stop** after 12 attempts, after 4 rejected in a row, or when the realistic score is below
30. Then do the final checks.

**Where gains are likely** (from the bang-bang baseline; check them, don't assume):
- Latency compensation: the robot acts on commands 33 ms late, and vision is 33 ms old.
  Predict the pose about 66 ms ahead from the commands already sent, and control that.
- The `sqrt(2·a·d)` braking curve has unbounded gain near the target. A linear (P) zone
  inside a few cm or degrees stops the hunting.
- Braking with a margin (plan the stop with a little less than `a_max`) for robots that can't
  reach `a_max`.
- Ramping from the measured velocity, or blending it with the last command, so the command
  doesn't run ahead of a weak robot.
- Turning the field→robot frame with the heading predicted for when the command acts, not
  the 66 ms old one.
- Online estimation of the acceleration the robot actually achieves, used as its `a_max`.
- While driving: limiting the turn rate, or turning first, so rotation doesn't bend the path.

# Final checks

1. **Robustness.** Run the final controller and the starting controller with `SIM_SEED=1`
   and `SIM_SEED=2` (realistic) through `track.py`. Report the mean ± spread. If the final one
   isn't better on the mean, say so plainly.
2. **New reference.** If the final controller beats the reference in `results/baseline_*.csv`:
   - Run both conditions with plain `run.lua`.
   - Copy `results/latest_ideal.csv` and `results/latest_realistic.csv` over the baselines.
   - Commit: `controller <name>: new benchmark baseline`.

   The next session will then start from it.
3. **Report.** Write `reports/controller-<name>-<YYYY-MM-DD>.md` with:
   - starting and final scores (ideal, realistic, total) and the per robot × test costs
     before → after
   - each kept change: what, why, score delta and MLflow run id
   - each rejected attempt: hypothesis, result, and why it didn't work
   - the robustness check
   - what still fails, and the next hypotheses you would try

   Commit it.

# Never

- Change the benchmark, the robots, the simulator or the engine to get a better score.
- Read the true state, the robot profile or the condition from inside the controller.
- Keep a change because of one lucky run: small gains need the seed check.
- Push, or touch other branches.
