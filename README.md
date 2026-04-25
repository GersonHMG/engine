# Sysmic Robotics — RoboCup SSL Engine

Since the old [engine](https://github.com/Sysmic-Robotics/condorssl) was to heavy to install I decided to migrate to a light version using Rust and Iced.

![Engine demo](docs/example.gif)

A Rust-based engine for RoboCup Small Size League (SSL) robots. It handles vision data (SSL-Vision), the game controller (ssl-game-controller), robot control via radio or grSim simulation, and exposes a Lua scripting interface for strategy code. A desktop GUI is built with [Iced](https://github.com/iced-rs/iced).

## Prerequisites

### Rust toolchain

Install Rust via [rustup](https://rustup.rs/):

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Requires **Rust 1.75+** (edition 2021). No external `protoc` binary is needed — protobuf code generation is handled by a pure-Rust build script.

### Linux system dependencies

On Debian/Ubuntu, install the libraries required by the GUI and serial-port crates:

```sh
sudo apt-get update
sudo apt-get install -y \
    pkg-config \
    libudev-dev \
    libxkbcommon-dev \
    libx11-xcb-dev \
    libxcb1-dev \
    libxcb-render0-dev \
    libxcb-shape0-dev \
    libxcb-xfixes0-dev \
    libxcb-icccm4-dev \
    libxcb-image0-dev \
    libxcb-keysyms1-dev \
    libxcb-randr0-dev \
    libxcb-render-util0-dev \
    libxcb-xinerama0-dev \
    libglvnd-dev \
    libgl1-mesa-dev \
    libwayland-dev \
    libdbus-1-dev
```

### Windows

No additional system dependencies are required on Windows.

## Building

### Debug build

```sh
cargo build
```

### Release build

```sh
cargo build --release
```

The compiled binary will be located at:
- Linux: `target/release/engine`
- Windows: `target\release\engine.exe`

## Running

```sh
cargo run --release
```

Or run the pre-built binary directly:

```sh
# Linux
./target/release/engine

# Windows
.\target\release\engine.exe
```

The engine opens a desktop GUI window. Use the sidebar panels to configure:

- **Vision** — SSL-Vision multicast address and port (default `224.5.23.2:10020`)
- **Control** — manual control
- **Radio** — serial port and baud rate for hardware robots
- **Recording** — log robot states to CSV
- **Kalman** — tracker filter settings

## Lua scripting

Strategy logic is written in Lua 5.4 scripts. Example scripts are provided in `scripts-test/`:

| Script | Description |
|---|---|
| `script.lua` | Minimal example: move robot 0 to the origin |
| `draw_demo.lua` | Square path following with visual overlay |

Load a script from the GUI toolbar or pass the path through the interface. The engine reloads the script on each tick, calling the `process()` function.

### Lua API reference

The engine exposes these functions to Lua scripts.

Conventions:
- `team`: `0` = blue, `1` = yellow
- Point/table arg: `{x = number, y = number}`
- Optional args marked with `?` use the default values shown below

#### Control and actuation

- `send_velocity(id, team, vx, vy, omega)`
    Sends direct velocity commands.
- `move_to(id, team, {x=, y=})`
    Move robot toward a target point using the default motion controller.
- `move_direct(id, team, {x=, y=})`
    Move robot directly toward a point (no path planning).
- `motion(id, team, {x=, y=}, kp_x?, ki_x?, kp_y?, ki_y?)`
    PID-like motion command with optional gains.
    Defaults: `kp_x=0.5`, `ki_x=0.1`, `kp_y=0.5`, `ki_y=0.1`.
- `face_to(id, team, {x=, y=}, kp?, ki?, kd?)`
    Rotates robot to face a target point.
    Defaults: `kp=1.0`, `ki=1.0`, `kd=0.1`.
- `bangbang_trajectory(id, team, v_max, a_max, {{x,y}, {x,y}, ...})`
    Follows a waypoint trajectory with bang-bang constraints.
    Can use indexed points (`{x, y}`) or keyed points (`{x=, y=}`).
- `kickx(id, team)`
    Trigger straight kick.
- `kickz(id, team)`
    Trigger chip kick.
- `dribbler(id, team, speed)`
    Set dribbler speed, clamped to `[0.0, 10.0]`.

#### State and referee

- `get_robot_state(id, team) -> table`
    Returns: `{id, team, x, y, vel_x, vel_y, orientation, omega, active}`.
- `get_ball_state() -> table`
    Returns: `{x, y, vel_x, vel_y}`.
- `get_blue_team_state() -> table[]`
    Array of robot state tables for the blue team.
- `get_yellow_team_state() -> table[]`
    Array of robot state tables for the yellow team.
- `get_ref_message() -> string`
    Returns current referee message/state as a string.

#### Field drawing helpers

- `draw_point(x, y)`
    Draw a point marker on the field view.
- `draw_line({{x,y}, {x,y}, ...}[, draw_points_between][, color])`
    Draw a polyline on the field view (2 or more points).
    Can use indexed points (`{x, y}`) or keyed points (`{x=, y=}`).
    `draw_points_between` is optional (`true`/`false`) and draws small interpolated points between line vertices.
    `color` is optional and can be `{r,g,b}` or `{r=, g=, b=}` with values in `[0.0, 1.0]`.
- `highlight_robot(id, team)`
    Highlight a robot in the field view.

#### grSim helpers

- `grsim.teleport_robot(id, team, x, y, dir)`
    Teleport robot in simulation.
- `grsim.teleport_ball(x, y)`
    Teleport ball in simulation.

## Project structure

```
engine/
├── build.rs                  # Protobuf codegen (pure Rust, no protoc needed)
├── Cargo.toml
├── scripts-test/             # Example Lua strategy scripts
├── src/
│   ├── main.rs               # Entry point — Iced GUI + async engine loop
│   ├── game_controller.rs    # ssl-game-controller client
│   ├── world.rs              # Shared game state
│   ├── lua_interface.rs      # Lua 5.4 scripting bridge
│   ├── receiver/             # Receiver-side modules
│   │   ├── vision.rs         # SSL-Vision UDP multicast receiver
│   │   └── tracker.rs        # Kalman filter tracker
│   ├── sender/               # Sender-side modules
│   │   ├── radio.rs          # Serial radio dispatcher
│   │   ├── grsim.rs          # grSim UDP command sender
│   │   └── packet_serializer.rs # Serial packet encoding
│   ├── motion/               # Controllers (PID, bang-bang) and path planning
│   ├── gui/                  # Iced GUI panels and field canvas
│   └── protobuf/protos/      # SSL and grSim .proto definitions
└── build/                    # Pre-built web assets for GUI panels
```

## License

Apache License 2.0 — see [LICENSE](LICENSE).
