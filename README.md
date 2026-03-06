# Sysmic Robotics — RoboCup SSL Engine

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

Available Lua API functions (provided by the engine):

- `move_to(robotId, team, {x, y})` — send a move-to command
- `get_robot_state(robotId, team)` — returns `{x, y, heading, vx, vy}`
- `get_blue_team_state()` / `get_yellow_team_state()` — full team state
- `draw_line(coords, color)` — draw a polyline overlay on the field canvas
- `draw_circle(x, y, radius, color)` — draw a circle overlay

## Project structure

```
engine/
├── build.rs                  # Protobuf codegen (pure Rust, no protoc needed)
├── Cargo.toml
├── scripts-test/             # Example Lua strategy scripts
├── src/
│   ├── main.rs               # Entry point — Iced GUI + async engine loop
│   ├── vision.rs             # SSL-Vision UDP multicast receiver
│   ├── tracker.rs            # Kalman filter tracker
│   ├── game_controller.rs    # ssl-game-controller client
│   ├── world.rs              # Shared game state
│   ├── lua_interface.rs      # Lua 5.4 scripting bridge
│   ├── radio.rs              # Serial radio driver
│   ├── grsim.rs              # grSim UDP command sender
│   ├── motion/               # Controllers (PID, bang-bang) and path planning
│   ├── gui/                  # Iced GUI panels and field canvas
│   └── protobuf/protos/      # SSL and grSim .proto definitions
└── build/                    # Pre-built web assets for GUI panels
```

## License

Apache License 2.0 — see [LICENSE](LICENSE).
