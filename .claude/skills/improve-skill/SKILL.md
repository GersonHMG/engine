---
name: improve-skill
description: Improve a Lua robot skill in lua/skills/ by measuring it in the simulator. Use when asked to improve, tune or fix a skill.
---

Before testing, read `simulator/CLAUDE.md`: how to run skills headless in the simulator, how Lua commands map to it, and what it does not model. You run every test yourself; do not ask the user to run them.

# Rules & Constraints

1. **Language:** Lua 5.4 (the engine embeds Lua 5.4, so `math.atan(y, x)` is the two-argument arctangent).
2. **Single file scope:** edit only the file of the skill being improved. Do not create new files. Shared helpers already in `lua/utils/` may be required.
3. **Preserve external APIs:** use only the existing engine functions (`get_ball_state()`, `get_robot_state()`, `move_to()`, `move_direct()`, `face_to()`, `kickx()`, `dribbler()`, `draw_*`) without changing how they are called.
4. **Skill format:** every skill follows the format below. A change that breaks it is rejected, whatever its score.

# Skill Format

## Module

- The file `lua/skills/<name>.lua` defines `local <name> = {}` and ends with `return <name>`.
- The entry point is `function <name>.process(robotId, team, target)`. `target` is optional and is a point `{x = ..., y = ...}`. Do not add parameters: the actions in `lua/AI/actions/` call skills with this signature.
- The file starts with a header comment stating what the skill does, its parameters, and its termination condition in one sentence.
- Tunable values (tolerances, offsets, speeds) are `UPPER_CASE` local constants at the top of the file, with their unit in a comment. No magic numbers inside `process`.

## Termination condition

- Every skill has an explicit termination condition: a named local predicate, e.g. `local function is_done(robot, ball, target)`, that returns a boolean.
- It is evaluated from the observed world state (`get_robot_state`, `get_ball_state`, `has_the_ball`), not from a counter or a flag set by the skill itself.
- Geometric conditions use a named tolerance constant (`DIST_TOLERANCE`, `ANGLE_TOLERANCE`). The tolerance must be reachable: larger than what the robot's controller can settle to, otherwise the skill never ends.
- `process` returns `true` only on the tick the goal is accomplished, and `false` while in progress. It never returns `nil`.
- The termination check comes before any motion command, so a finished skill does not move the robot again.
- One-shot skills (e.g. `kick`) issue their command and return `true` on the same tick. A final action such as a kick may be issued on the tick the skill returns `true`; motion commands may not.

## Execution model

- `process` is called once per tick (60 Hz) and must return within that tick: no loops waiting on the world, no sleeps.
- While returning `false`, it issues a command every tick (a motion, `face_to`, or an explicit stop), so the robot never keeps a stale command.
- Prefer no module-level state. If state is unavoidable, key it by `team` and `robotId`, because several robots run the same skill at the same time.
- Phases (e.g. approach, then touch) are chosen from the current state each tick, never remembered.

## Math conventions

- Units: meters, seconds, radians; field coordinates with the origin at the center.
- Angles: use `math.atan(y, x)`, and normalize every angle difference to [-π, π] before comparing it with a tolerance (wrap in both directions, then take the absolute value).
- Guard every division by a distance or vector length against zero.
- `draw_*` debug calls are allowed; they must not change behavior.

## Example: `move`

```lua
-- move: drives the robot to `target`.
-- Parameters: robotId, team, target {x, y} (m).
-- Terminates when the robot is within DIST_TOLERANCE of target.
local move = {}

local DIST_TOLERANCE = 0.1 -- m

local function is_done(robot, target)
    return math.sqrt((robot.x - target.x)^2 + (robot.y - target.y)^2) <= DIST_TOLERANCE
end

function move.process(robotId, team, target)
    local robot = get_robot_state(robotId, team)

    if is_done(robot, target) then
        return true
    end

    move_to(robotId, team, target)
    return false
end

return move
```

# Loop
1. **Benchmark.** If `lua/tests/<skill>.lua` does not exist, write it following
   `simulator/CLAUDE.md` (several setups, one line per setup, a summary, `sim.finish`).
   Measure outcomes from the world and the simulator events, never from the skill's
   return value. Once the baseline is recorded, the test is frozen.
2. **Baseline.** Run `./target/debug/engine --headless lua/tests/<skill>.lua` (run
   `cargo build` first if the binary is missing). Record the output.
3. **Hypothesis.** Read the results and the code, and form ONE hypothesis. Write down
   which setups you expect to change and roughly by how much.
4. **Change.** Make one change to `lua/skills/<skill>.lua` only.
5. **Measure.** Rerun the benchmark. Keep the change if the score improves without
   breaking a constraint; otherwise revert it. A run that exits with 2 (Lua error)
   is a failed change: fix or revert it.
6. Repeat from 3. Stop after 10 changes or 3 rejected in a row.

# Never
- Edit the test after the baseline, scenarios/, simulator.toml or engine code — that changes the exam, not the skill.
- Loosen a termination tolerance just to finish sooner: report it as a proposal instead.

# Report
Write reports/<skill>-<date>.md with: baseline vs final score per scenario, each change
kept (with the reason and the score delta), each change rejected, and remaining failures.