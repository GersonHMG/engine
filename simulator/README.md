# SSL Simulator

A deterministic 2D simulator for RoboCup SSL, written to run faster than
real time and to be driven by agents. It runs on Linux and Windows.

Current scope: **one robot, one ball, a field closed by walls.**

## Layout

```
crates/
  sim/       ssl-sim            physics library: no I/O, no globals, no randomness
  scenario/  ssl-sim-scenario   scenario files and their command script
  cli/       engine-sim         runs scenario files, prints JSON (headless)
  gui/       engine-sim-gui     small window showing the field
scenarios/                      example scenario files
```

The GUI is a separate binary so the CLI stays free of graphics
dependencies: it builds on headless machines and in CI.

## Usage

```sh
cargo test --workspace
cargo sim run scenarios/drive_and_kick.toml --pretty
cargo sim run scenarios/drive_and_kick.toml --replay run.csv
cargo gui scenarios/drive_and_kick.toml
cargo gui
```

`cargo sim` and `cargo gui` are aliases defined in `.cargo/config.toml`.

## GUI

`cargo gui <scenario>` opens a small window with the field and plays the
scenario in real time until its duration. `cargo gui` alone shows the
default starting position.

On Linux the GUI needs the usual windowing libraries (X11 or Wayland
development packages); the CLI and the library do not.

`engine-sim run` prints a JSON report on stdout: the final robot and ball
state plus the events (`kick`, `ball_hit_wall`, `robot_hit_wall`,
`robot_touched_ball`) with their times. Errors go to stderr with a non-zero
exit code.

`--replay` also writes the trajectory as CSV in the engine logger's format,
so it can be opened in the engine GUI's replay panel.

## Running it inside the engine

The engine can drive the simulator instead of grSim. Your Lua AI controls
the robot and the engine GUI shows it live.

Start the engine as usual (`cargo run` from the repository root) and click
**SIM** in the toolbar. A speed menu (1x, 2x, 5x, 10x, Max) appears next
to it. Click **SIM** again to go back to grSim or real robots: the vision
receiver restarts.

To start with the simulator already on:

```sh
cargo run -- --sim lua/run_ai.lua
cargo run -- --sim --speed 4 lua/run_ai.lua
```

The script loads paused; press play in the GUI.

Type `sim ...` commands in the command line at the bottom of the Lua
console:

| Command | Effect |
|---|---|
| `sim help` | list the commands |
| `sim status` | time, every robot and the ball |
| `sim add <blue\|yellow> <id> [x y [θ]]` | add a robot (default: a free spot on its side) |
| `sim remove <blue\|yellow> <id>` | remove a robot |
| `sim reset` | robots and ball back to where they were last placed |
| `sim config` | all parameters (field, robot, ball) |
| `sim robot [<parameter> <value>]` | show or change the robot profile, e.g. `sim robot max_speed 2.5` |
| `sim ball [<parameter> <value>]` | show or change the ball profile, e.g. `sim ball roll_decel 0.5` |
| `sim save` | keep the current robot and ball profiles in `simulator.toml` |
| `sim scenario save <name>` | save where every robot and the ball are, in `scenarios/<name>.toml` |
| `sim scenario load <name>` | put the robots and ball back as saved (other robots are removed) |
| `sim scenario list` | saved scenarios |

On the field, left-click a robot to select it (Escape deselects). Right-click
opens a menu: move the ball there, teleport the selected robot there, or
add a blue or yellow robot there (with the lowest free id).

The parameters persist in `simulator.toml` at the repository root, read
every time SIM is turned on. Edit it by hand and toggle SIM off and on, or
tune live with `sim robot` / `sim ball` and then `sim save`. Field length
and width come from `config.ini`.

Teleporting a robot that is not simulated yet (from Lua or the right-click
menu) adds it, as grSim does when it turns a robot on. The simulated robot is
blue robot 0; commands and teleports for other robots are ignored. In
simulator mode the vision receiver is off and `grsim.teleport_robot` /
`grsim.teleport_ball` move the simulated robot and ball.

## Scenario files

```toml
name = "drive_and_kick"
duration = 5.0          # simulated seconds
tick = 0.016667         # control period, default 1/60 s

[robot]
x = -1.0                # m; theta in rad, 0 = facing +x
y = 0.0
theta = 0.0

[ball]
x = 0.0                 # vx, vy optional
y = 0.0

[[commands]]            # in effect from t until the next entry
t = 0.0
vt = 1.0                # forward, m/s (robot frame, grSim convention)
vn = 0.0                # left, m/s
omega = 0.0             # rad/s
kick_speed = 4.0        # m/s, fires once when the ball reaches the kicker

[config.ball]           # optional overrides, see crates/sim/src/config.rs
roll_decel = 0.35
```

## Physics model

| Part | Model |
|---|---|
| Field | SSL division B: 9 × 6 m playing area, 0.3 m boundary, walls at the outer edge |
| Robot | circle of radius 0.09 m cut flat at 0.073 m (front face). Holonomic: tracks the commanded velocity within acceleration limits. Infinite mass relative to the ball. Walls stop it. |
| Ball | radius 0.0215 m. Slides at 3.0 m/s² until it has slowed to 5/7 of its speed after an impulse, then rolls at 0.35 m/s². Walls bounce it (restitution 0.6). |
| Robot vs ball | the ball is pushed out of the body and bounces off it (restitution 0.5 on the body, 0.2 on the front face), including the body's rotation |
| Kicker | `kick_speed` fires once when the ball is at the front face (up to 1 cm in front of it): ball velocity = robot velocity + heading × kick speed |
| Integration | semi-implicit Euler, substeps of at most 1 ms |

All parameters live in `SimConfig` and can be overridden per scenario.

## Determinism

The simulator uses no randomness and computes trigonometry with `libm`
instead of the platform math library, so the same scenario gives the same
result on Linux and Windows. `Simulator::state().clone()` is a full
snapshot: `restore` replays the same trajectory exactly.
