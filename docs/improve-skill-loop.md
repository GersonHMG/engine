# Improving a skill: the measurement loop

How an agent improves a Lua skill in `lua/skills/` by measuring it in the headless
simulator. This is a draft refinement of `.claude/skills/improve-skill/SKILL.md`: the
skill format and the coding rules stay as written there. This file defines the
objective, the scenarios, the observations and the loop.

Read `simulator/CLAUDE.md` first: how headless runs work, the `sim` table, the
`engine` MCP tools, and what the simulator does not model.

---

## Starting a session

Open a new Claude Code session at the repository root and send:

```
/improve-skill <skill>

Follow docs/improve-skill-loop.md.
Objective: <metric, e.g. time of the kick event from the start of each scenario>.
Constraints: <e.g. every scenario that succeeds in the baseline still succeeds;
angle error no worse than the baseline + 0.05 rad>.
Tuning scenarios: <list them, or "write them from the skill's purpose">.
Holdout: <e.g. a grid of distance x lateral offset x target angle, ~90 setups>.
Callers to keep working: <e.g. lua/AI/actions/pass.lua, kick_to_goal.lua, kick_to_clear.lua>.
Work autonomously; show me the worst holdout case in the engine at the end.
```

Before sending it:

- `cargo build` once, so `target/debug/engine.exe` exists.
- If you want the agent to show you scenarios, open the engine GUI (it serves the
  `engine` MCP on port 7878) and approve the `engine` server when Claude Code asks.
  Measurement does not need it.

## Session folder

Everything a session produces lives in one folder, ignored by git (`sessions/` is in
`.gitignore`). Only the improved skill, `lua/skills/<skill>.lua`, is changed in the
repository; you review and commit it yourself.

```
sessions/<skill>-<YYYY-MM-DD>-<HHMM>/
  objective.md        metric, constraints, decision rule, failure cost (section 0)
  test.lua            tuning + holdout scenarios and the observations; frozen after the baseline
  show.lua            runs one scenario in the GUI, for the engine MCP (section 3.6)
  skill.orig.lua      the skill as it was at the start
  baseline/
    stdout.txt        summary + digest of the baseline run
    traj/<scenario>.csv
  runs/<NN>-<slug>/   one folder per change, NN = 01, 02, ...
    hypothesis.md     cause + evidence, change, prediction per scenario, risk
    skill.diff        the change, as a diff against the previous kept version
    stdout.txt        summary + digest of this run
    traj/<scenario>.csv
    verdict.md        prediction vs result, kept or reverted, and why
  report.md           the final report (section 6)
```

- The agent creates the folder at the start and writes its path as `SESSION` at the
  top of `test.lua`; the test writes every output file under it.
- Run it from the repository root, so `require("skills.<skill>")` resolves:
  `./target/debug/engine.exe --headless sessions/<id>/test.lua > sessions/<id>/runs/<NN>-<slug>/stdout.txt`
- A reverted change keeps its run folder: the record of what did not work is part of
  the result.
- To keep a report in the repository, copy `report.md` to `reports/` yourself.

## 0. Objective (written once, before any scenario)

Write these at the top of the test file, as comments, before running anything:

- **Metric:** one number to improve, defined from the world and the simulator
  events, never from the skill's return value.
  Example: time of the `kick` event from the start of each scenario, summed.
- **Constraints:** pass/fail rules a change must keep.
  Example: every scenario that succeeds in the baseline still succeeds; angle error
  does not get worse than the baseline by more than 0.05 rad in any scenario.
- **Decision rule:** keep a change only if the metric improves on the tuning set,
  no constraint breaks on the tuning set, and the holdout aggregate does not get
  worse (section 1).
- **Failure cost:** what a failed scenario counts as in the metric (e.g. the time
  limit), so a failure is never cheaper than a slow success.

## 1. Scenarios: two sets

| Set | What it is | What the agent sees |
|---|---|---|
| **Tuning** | 4–8 named, hand-picked setups that cover the typical cases | Everything: summary, digest, trajectory files |
| **Holdout** | A generated sweep (grid or fixed-seed random), 50–200 setups, including hard cases | Only the aggregate and the worst cases (section 3) |

- Write the scenarios **before reading the skill's code**, from what the skill is
  supposed to do, so the tests are not shaped by what the current code already
  handles.
- Cover the hard cases on purpose: sharp angles, very close and very far starts,
  a moving ball, a robot near a wall, an opponent in the way.
- Both sets are **frozen after the baseline**. A bug in the test found before any
  change to the skill may be fixed, and then the baseline is recorded again.

Why two sets: in the kick_to_point run of 2026-10-06, the tuning set improved by 21 %
while the holdout dropped from 90/90 to 74/90 kicks. Without a holdout, that
regression would have shipped.

## 2. Baseline

1. Copy the skill to `skill.orig.lua`.
2. Run the test with the unchanged skill, writing to `baseline/`.
3. Run it again and check that the output is identical (the simulator is deterministic;
   a difference means the test depends on something it should not).
4. From now on `test.lua` is frozen.

## 3. Observations

Three layers. The first two are printed on every run; the third is written to disk
and read only when needed, so it does not fill the agent's context.

### 3.1 Summary (stdout, every run)

```
SUMMARY tuning: ok=5/5 metric=4.268 s   constraints: PASS
SUMMARY holdout: ok=89/90 metric=131.8 s   worst: sweep_035 (5.218 s), sweep_053 (fail), sweep_027 (2.419 s)
```

### 3.2 Per-scenario digest (stdout, every run, tuning set only)

One line per scenario, with the outcome plus numbers that point at the cause:

```
offset_close  ok  t=0.621 s  t_near=0.250 s  err=0.030 rad  phases: approach 0.47 s, touch 0.15 s  switches=1  touches_before=0  contact_speed=1.00 m/s
```

| Field | Meaning | A bad value says |
|---|---|---|
| outcome, time, accuracy | the metric and the constraints | |
| intermediate times | e.g. time until the robot is within 0.25 m of the ball | where the time goes |
| time per phase | from the `phase` column (3.3) | which phase is slow |
| `switches` | number of phase changes | many: the skill hesitates between phases |
| `touches_before` | ball touches before the success event | > 0: the robot bumped the ball instead of kicking it |
| skill-specific at the event | e.g. speed, lateral offset, heading error at contact | how close it was to failing |

### 3.3 Trajectory file (disk, every run, read on demand)

**Path:** `<run folder>/traj/<scenario>.csv` (`baseline/` or `runs/<NN>-<slug>/`).
Holdout scenarios use `sweep_<NNN>.csv`.

**Rows:** one per tick (1/60 s), from the start of the scenario to the end of its
measurement. Every tick, never sampled: a fast contact happens within 1–2 ticks.

**Header:** the scenario and its setup as comment lines, so each file stands alone:

```
# scenario=offset_close robot=(-0.30,0.20,-0.588) ball=(0.00,0.00) target=(4.50,0.00)
tick,t,rx,ry,rth,rvx,rvy,rw,bx,by,bvx,bvy,dist,phase,reason,returned,event
37,0.617,-0.107,-0.012,-0.014,0.99,-0.15,0.00,0.000,0.000,0.00,0.00,0.108,approach,facing_fail 0.126>0.100,false,
```

**Columns** (meters, seconds, radians, field frame):

| Column | Meaning |
|---|---|
| `tick` | tick number since the scenario started |
| `t` | seconds since the scenario started |
| `rx, ry, rth` | robot position and heading |
| `rvx, rvy, rw` | robot velocity and angular velocity |
| `bx, by, bvx, bvy` | ball position and velocity |
| `dist` | robot center to ball center |
| `phase` | the branch of the skill that ran this tick |
| `reason` | the check that chose that branch, with its value and tolerance |
| `returned` | what `process()` returned this tick |
| `event` | simulator events during this tick: `kick(3.00)`, `touch(front)`, `touch(body)`, `robot_wall`, `ball_wall`, or empty |

#### `phase` and `reason`

`phase` is the branch of `process()` that ran: every skill chooses its phase from the
current state each tick (skill format), and this column records the choice.
`reason` names the check that decided it, with the measured value and the tolerance,
so a near miss is visible.

Example for kick_to_point:

| `phase` | Branch | Commands |
|---|---|---|
| `approach` | not lined up | `face_to` + `move_to` the point behind the ball |
| `touch` | lined up, ball not yet at the kicker | `face_to` + `move_direct` past the ball |
| `done` | `is_done` true | `kickx`, no motion |

| `reason` examples |
|---|
| `line_fail 0.080>0.050` |
| `facing_fail 0.126>0.100` |
| `behind_fail` |
| `lined_up` |
| `ball_at_kicker` |

**How the test reads it.** The test cannot see inside `process()`, so the skill
publishes its choice in a debug table, keyed by team and robot (the only module-level
state the skill format allows). The skill writes it every tick and never reads it:

```lua
-- Observation only: the phase chosen on the last tick, per robot. Never read by the skill.
kick_to_point.debug = {}

local function report(robotId, team, phase, reason)
    kick_to_point.debug[team] = kick_to_point.debug[team] or {}
    kick_to_point.debug[team][robotId] = { phase = phase, reason = reason }
end
```

The signature `process(robotId, team, target)` does not change, so the callers in
`lua/AI/actions/` are unaffected.

#### `returned`

The value `process()` returned on that tick: `true` means the skill claims it is
done, `false` means it is still working. It is an **observation, not an outcome**:
success is always measured from the world (the `event` column). Comparing the two
finds termination bugs:

| Pattern | Meaning |
|---|---|
| `returned=true` and no success event follows | the skill claims a success that did not happen (e.g. it stops short of the ball) |
| success event while `returned=false` | the skill missed its own completion; callers would keep driving the robot |
| `returned` flips true/false/true | unstable termination condition |

### 3.4 Holdout observations

The agent sees only:

- the aggregate line (ok count, metric),
- the 3 worst scenarios (failed first, then slowest), each with its digest line.

It may open the trajectory files of those 3. It does not tune to individual holdout
scenarios; a holdout failure is a symptom to explain with a hypothesis, and the fix
must be general.

### 3.5 Writing the files

The engine creates its Lua state with `Lua::new()` (mlua's safe standard libraries),
so `io.open` and `os.getenv` are available to test scripts. Buffer the rows in a table
and write them once at the end of each scenario with `io.open(path, "w")`. The output
folder is passed in an environment variable, so the frozen test can write each run to
its own folder:

```lua
local SESSION = "sessions/kick_to_point-2026-10-06-1530"            -- written once, at creation
local OUT = os.getenv("RUN_DIR") or (SESSION .. "/baseline")          -- where this run writes
```

Lua cannot create folders: the agent creates `<run folder>/traj/` before each run.

### 3.6 Watching a scenario in the engine (MCP)

The headless run is the measurement. The `engine` MCP drives the user's open GUI in
real time, so it is used to **look** at a scenario, never to score one: it is slower,
it runs in real time, and it moves things in the user's window.

Use it when:

- a trajectory file does not explain a failure, and seeing the motion would;
- at the end of the session, to show the user the worst holdout case before and
  after the change.

How: `show.lua` runs the skill on one scenario without the headless-only `sim` table
(the GUI does not have it). It reads the scenario to show from a small file the agent
writes, `sessions/<id>/show_scenario.lua` (`return { robot = {...}, ball = {...}, target = {...} }`),
then:

1. `set_simulator(enabled=true, speed=1)`
2. `teleport_ball` and `teleport_robot` to the scenario's start
3. `load_script("sessions/<id>/show.lua")`, then `play_script`
4. `wait(seconds)` and `get_log` to follow it; `pause_script` when done

Do not write scenarios into `scenarios/`: that folder is committed. If the engine is
not open, the MCP tools fail; skip the showing and say so in the report.

## 4. The loop

For each change:

1. **Read** the summary and the digest. Open a trajectory file only to explain a
   specific number (a slow phase, a touch before the kick, a failure).
2. **Hypothesis.** Write down, before changing anything:
   - the cause, citing the evidence (a trajectory row, a digest field),
   - the change,
   - the expected effect per tuning scenario, with a rough size,
   - the risk: which scenarios could get worse and why.
3. **Change** one thing in `lua/skills/<skill>.lua`.
4. **Run** the test (tuning and holdout in the same run).
5. **Compare with the prediction**, not only with the score. If the result differs
   from the prediction, the cause was wrong: say so, and read a trajectory before the
   next hypothesis.
6. **Decide** with the decision rule (section 0): keep, or revert. A run that exits
   with code 2 (Lua error) is a failed change.

**Stop** after 10 changes, after 3 rejected changes in a row, or when 3 kept changes
in a row each improve the metric by less than 1 %.

## 5. Rules

- Do not edit `sessions/<id>/test.lua`, `objective.md`, `simulator.toml` or engine
  code after the baseline. That changes the exam, not the skill.
- In the repository, change only `lua/skills/<skill>.lua`. Everything else the session
  writes goes in its session folder; nothing is committed by the agent.
- Do not loosen a termination tolerance to finish sooner; report it as a proposal.
- Changes beyond the budget or the rules are written in the report as proposals,
  with any measurements, and are not applied.
- The simulator has perfect sensing and no dribbler: prefer margins over precise
  tuning (see `simulator/CLAUDE.md`).

## 6. Report

Write `sessions/<id>/report.md` with:

- the objective and constraints,
- baseline vs final per tuning scenario (metric and constraints),
- the holdout aggregate, baseline vs final,
- every change: hypothesis, prediction, result, kept or rejected,
- remaining failures, with the trajectory evidence,
- proposals that the budget or the rules did not allow,
- the path of the session folder, so every number in the report can be traced to a run.
