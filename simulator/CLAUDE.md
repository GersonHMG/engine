# Using the simulator (agent context)

A deterministic 2D RoboCup SSL simulator, written in Rust. It runs faster than real
time and is the place to test robot behavior. Units: meters, seconds, radians.
Field coordinates have the origin at the center, and θ = 0 faces +x.

Choose by what you need to test:

| Need | Use |
|---|---|
| Lua code: skills, actions, roles, `lua/AI` | the engine with `--headless` (section 1) |
| Physics, or a fixed sequence of velocity/kick commands | the `engine-sim` CLI (section 2) |
| Watching a behavior, or showing it to the user | the engine GUI with `--sim` (section 3) |

Sections 1 and 2 need no person and finish in a fraction of a second: run them
yourself, as often as you need.

## 1. Headless Lua runs (the main tool)

From the repository root:

```sh
cargo build                                              # once; Lua changes need no rebuild
./target/debug/engine --headless lua/tests/<name>.lua    # Windows: target\debug\engine.exe
./target/debug/engine --headless --max-time 120 lua/tests/<name>.lua
```

The engine runs the script exactly as the GUI would (each tick: the script's
`process()`, then one simulator step of 1/60 s) but with no window and as fast as
the machine allows: a minute of play takes well under a second.

- The script's `print` goes to **stdout**; engine logs and errors go to **stderr**.
  The last stderr line is a summary: exit code, simulated time, ticks, real time.
- Exit codes: `0` the script called `sim.finish()` or `sim.finish(true)`, `1` it
  called `sim.finish(false)`, `2` the script failed to load or raised an error
  (the Lua traceback is on stderr), `3` the time limit was reached (default 60 s
  simulated, change it with `--max-time`).
- Lua files are read at run time, so after editing a `.lua` file just run again.
  Rebuild only after changing Rust code. If `cargo build` fails with "Access is
  denied", the user has the engine GUI open: ask them to close it, or keep using
  the existing binary for Lua-only work.

Headless scripts get a `sim` table on top of the normal API:

| Function | Returns |
|---|---|
| `sim.time()` | simulated time, s, since the run started |
| `sim.events()` | a list of the simulator events since the last call (see below) |
| `sim.finish(ok)` | ends the run after this tick; `ok` defaults to `true`, `false` exits with 1 |

| Event `kind` | Fields (besides `time`) |
|---|---|
| `kick` | `id`, `team`, `speed` |
| `robot_touched_ball` | `id`, `team`, `on_front` (true = on the kicker face) |
| `robot_hit_wall` | `id`, `team`, `x`, `y` |
| `ball_hit_wall` | `x`, `y`, `speed` (before the bounce) |

`team` is 0 for blue, 1 for yellow, as in the rest of the Lua API.

### Writing a test script

Put test scripts in `lua/tests/`. A test runs several setups one after another in
the same run, measures each one, prints one line per setup and a summary, and
ends with `sim.finish`:

```lua
-- Tests in lua/tests/ need lua/ on the module path to require skills.
package.path = "lua/?.lua;" .. package.path
local skill = require("skills.kick_to_point")

local TIME_LIMIT = 10 -- s per setup
local setups = {
    { name = "aligned_close", robot = { x = -0.3, y = 0 }, target = { x = 4.5, y = 0 } },
    { name = "aligned_far",   robot = { x = -2.0, y = 0 }, target = { x = 4.5, y = 0 } },
}

local current, start

local function begin(index)
    current = index
    local setup = setups[index]
    -- Face the ball, at rest.
    grsim.teleport_robot(0, 0, setup.robot.x, setup.robot.y, math.atan(-setup.robot.y, -setup.robot.x))
    grsim.teleport_ball(0, 0)
    sim.events() -- drop the events of the previous setup
    start = sim.time()
end

local function next_setup(result)
    print(string.format("%-14s %s", setups[current].name, result))
    if current == #setups then
        sim.finish(true)
    else
        begin(current + 1)
    end
end

begin(1)

function process()
    local setup = setups[current]
    skill.process(0, 0, setup.target)
    for _, event in ipairs(sim.events()) do
        if event.kind == "kick" then
            return next_setup(string.format("kicked at %.3f s", event.time - start))
        end
    end
    if sim.time() - start > TIME_LIMIT then
        next_setup("no kick")
    end
end
```

Things to know when writing tests:

- Blue robot 0 exists from the start at (-1, 0). Teleporting any other robot adds
  it. Robots stay until the end of the run, so move or reuse robots of earlier
  setups so they do not get in the way.
- Teleports put robots and the ball at rest and take effect immediately.
- Measure outcomes from the world and the events (ball speed, `kick` events, robot
  positions), not from what the code under test returns: a skill can return
  `true` without having done anything.
- Runs are deterministic: the same script and files always print the same output.

## 2. Standalone CLI (physics only)

Run from `simulator/`:

```sh
cargo sim run scenarios/<name>.toml           # JSON report on stdout
cargo sim run scenarios/<name>.toml --pretty  # indented JSON
cargo sim run scenarios/<name>.toml --replay run.csv  # trajectory, opens in the engine replay panel
cargo test --workspace                        # physics tests
```

A scenario has one robot (blue 0), the ball and a time-stamped command script:

```toml
name = "drive_and_kick"
duration = 5.0          # simulated seconds
tick = 0.016667         # control period, default 1/60 s

[robot]
x = -1.0
y = 0.0
theta = 0.0

[ball]
x = 0.0                 # vx, vy optional
y = 0.0

[[commands]]            # in effect from t until the next entry
t = 0.0
vt = 1.0                # forward, m/s (robot frame)
vn = 0.0                # left, m/s (robot frame)
omega = 0.0             # rad/s
kick_speed = 4.0        # m/s; fires once when the ball reaches the kicker

[config.robot]          # optional overrides of the physics parameters
max_speed = 2.5
```

The report has `final` (every robot's position, θ, velocity, ω, and the ball's
position, velocity, speed and `sliding`) and `events` with their `time`:

| Event | Fields |
|---|---|
| `kick` | `robot`, `speed` |
| `ball_hit_wall` | `position`, `speed` |
| `robot_hit_wall` | `robot` |
| `robot_touched_ball` | `robot`, `on_front` (true = touched on the kicker face) |

Errors go to stderr with a non-zero exit code. The same scenario always gives the
same output on every machine, so two runs that differ mean the inputs differ.

## 3. Engine GUI (for watching)

From the repository root:

```sh
cargo run -- --sim lua/run_ai.lua             # real time
cargo run -- --sim --speed 4 lua/run_ai.lua   # 4x; --speed 0 = as fast as possible
```

This opens a window: use it to show a behavior to the user, not to test it. The
script loads paused until someone presses play. Every tick the engine calls the
script's global `process()` and then advances the simulator by exactly 1/60 s, at
any speed. Lua code sees the same API as with real robots (`lua/api_globals.lua`);
the `sim` table exists only in headless runs.

The entry script sets up the field once, at load time:

```lua
grsim.teleport_robot(id, team, x, y, theta)  -- team: 0 = blue, 1 = yellow; adds the robot if missing
grsim.teleport_ball(x, y)
```

### How Lua commands reach the simulator (headless and GUI)

| Lua call | In the simulator |
|---|---|
| `move_to`, `move_direct`, `face_to`, `send_velocity` | a robot-frame velocity (vx forward, vy left, ω), limited by `max_speed`, `max_accel`, `max_omega`, `max_alpha` |
| `kickx` | a flat kick at 3.0 m/s, fired when the ball reaches the kicker face |
| `kickz` | **ignored**: chip kicks are not modeled |
| `dribbler` | **ignored**: there is no dribbler |
| a robot with no command this tick | stops (decelerates to rest) |

### Lua console commands

Typed in the command line under the engine's Lua console:

| Command | Effect |
|---|---|
| `sim status` | time, every robot and the ball |
| `sim add <blue\|yellow> <id> [x y [θ]]` / `sim remove <team> <id>` | add or remove robots |
| `sim reset` | robots and ball back to where they were last placed |
| `sim config` | every physics parameter |
| `sim robot <param> <value>` / `sim ball <param> <value>` | change a parameter live |
| `sim save` | write the current parameters to `simulator.toml` |
| `sim scenario save\|load <name>` / `sim scenario list` | field setups in `scenarios/<name>.toml` at the repository root |

These field setups are a different format from the CLI scenarios in
`simulator/scenarios/`: they only hold positions (`[ball] x y`, `[[robots]] team id x y theta`),
with no commands.

## Physics model and its limits

| Part | Model |
|---|---|
| Field | 9 × 6 m (from `config.ini`), 0.3 m boundary, closed by walls. **No goals**: the ball bounces off the wall behind the goal line. |
| Robot | circle of radius 0.09 m, flat front face at 0.073 m. Holonomic, tracks the commanded velocity within its acceleration limits. Infinite mass relative to the ball. |
| Ball | slides (3.0 m/s²) after an impulse until it reaches 5/7 of its speed, then rolls (`roll_decel` in `simulator.toml`). Walls bounce it with restitution 0.6. |
| Robot vs ball | the ball bounces off the body (restitution 0.5) or the front face (0.2) |
| Robot vs robot | robots push each other apart |

The parameters live in `simulator.toml` at the repository root (the CLI uses the
defaults in `simulator/crates/sim/src/config.rs` plus each scenario's overrides).

Keep these limits in mind before trusting a result for the real robots:

- **No dribbler and no chip kick.** `has_the_ball` (`lua/utils/utils.lua`) is only
  geometry: the ball within 0.12 m and 0.35 rad of the robot's heading. Nothing holds
  the ball there, so driving with it pushes it away. A skill that relies on dribbling
  cannot be judged here.
- **Perfect information.** No vision noise, latency or dropped frames, and no
  wheel slip. A behavior that only just works in the simulator will likely fail on
  the field. Prefer margins over precise tuning.
- **Deterministic.** One run per setup tells you everything about that setup; to
  test robustness, vary the starting positions instead of repeating a run.

## Where things are

| Path | What |
|---|---|
| `simulator/crates/sim/` | physics library (`ssl-sim`) |
| `simulator/crates/scenario/`, `simulator/crates/cli/` | CLI scenario format and `engine-sim` |
| `simulator/crates/gui/` | standalone viewer (`cargo gui <scenario>`) |
| `src/headless.rs`, `src/lua_interface/api/sim.rs` | headless runner and the `sim` table |
| `src/sender/simulator/` | engine link: command mapping, console commands, field setups |
| `lua/tests/` | headless test scripts |
| `simulator.toml` | physics parameters used by the engine |
| `scenarios/` | field setups saved from the engine |
| `lua/api_globals.lua` | the Lua API available to scripts |
