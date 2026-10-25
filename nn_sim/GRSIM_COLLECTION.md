# Collecting robot-dynamics data from grSim

Goal: record one grSim robot driven by the same kind of commands used in session 1, so we can learn
grSim's robot dynamics. The engine drives the robot through the collection script
(`lua/tests/collect_dynamics.lua`), which writes its own CSV log with one row per tick (60 Hz):
the pose from vision, the tracker's velocities, the command sent, and a wall-clock timestamp.

Total time: about **1 h 45 min** of recording, which can be split across several sittings, plus setup.

---

## 1. One-time setup (on the PC with grSim)

1. **Get the code** (branch `nn-sim-session1`) and build the engine from the repository root:
   ```sh
   cargo build --release
   ```
   A rebuild is required: the script uses the new Lua function `wall_time()`. If the script stops with
   "wall_time() is missing", the engine binary is old.
2. **Python** (for the quality check): Python 3 with `numpy` and `pandas`.
3. **grSim settings** (check them once and write them down):
   - Field: **Division B (9 × 6 m)**, as in the engine's `config.ini`.
   - Vision multicast **224.5.23.2**, port **10020**; command listen port **20011**. These are grSim's defaults and what the engine uses.
     grSim must run on the same PC as the engine.
   - Blue robot **id 0** must exist. The other robots can stay; the script parks them on the edge of the field.
   - Vision noise, vanishing and delay: leave as they are, but **write down** whether they are on.
     The first recordings are best with noise and vanishing **off**.
   - Do not change grSim's speed or FPS during the recordings.
4. **Engine settings:** SIM button **off** (the engine must listen to grSim, not to its built-in simulator). Radio off (the default).

## 2. Pilot recording (5 minutes, do this first)

1. Start grSim.
2. Edit `lua/tests/collect_mix.lua` and set `duration_min = 2` and `seed = 99`.
3. From the repository root, start the engine with the script:
   ```sh
   ./target/release/engine lua/tests/collect_mix.lua        # Windows: target\release\engine.exe lua\tests\collect_mix.lua
   ```
   The script loads **paused**; press **Play**. You will see the robot jump to the center, drive, jump back, and so on.
   The Lua console shows `[collect] writing logs/dyn_mix_..._seed99.csv` at the start and `[collect] done` at the end.
4. Check the file (from the `nn_sim/` folder):
   ```sh
   python -m nn_sim.grsim_data check ../logs/dyn_mix_*_seed99.csv
   ```
5. **Send me the checker's output** before the long recordings. It reports tick timing, vision dropouts, the response delay
   and how different grSim is from our simulator. If it says `RESULT: CHECK -> ...`, fix that first (see section 5).

## 3. The recordings

For each row: edit the wrapper script (`seed`, `duration_min`), start the engine with it as above, press Play, wait for
`[collect] done`, then run the checker on the new file.

| # | Script | `seed` | `duration_min` | Used for |
|---|---|---|---|---|
| 1 | `lua/tests/collect_mix.lua` | 1 | 15 | training |
| 2 | `lua/tests/collect_mix.lua` | 2 | 15 | training |
| 3 | `lua/tests/collect_mix.lua` | 3 | 15 | training |
| 4 | `lua/tests/collect_mix.lua` | 4 | 15 | validation |
| 5 | `lua/tests/collect_mix.lua` | 5 | 15 | test (same kind of driving) |
| 6 | `lua/tests/collect_waypoint.lua` | 101 | 15 | test (AI-like driving) |

Each recording takes about 17 minutes of wall time (15 min of driving plus the resets).
If time is short, do **1, 2, 5 and 6** first; that is enough for a first model.

**Use a different seed for every recording.** The seed is in the file name, so a repeated seed is easy to spot.

During a recording:
- Do not move robots or the ball, and do not click on the field.
- Keep the PC otherwise idle (no heavy programs): late engine ticks show up as timing warnings in the checker.
- To stop early, press Pause/Stop or close the engine. The file is flushed every second, so a partial recording is still usable.

## 4. What to bring back

From the `logs/` folder: every `dyn_*.csv` together with its `dyn_*.json` (same name).
Also include:
- grSim's settings file (on Linux `~/.grsim.xml`), or screenshots of its settings pages. It holds the robot model
  (mass, wheel angles, motor limits), which helps explain what the network learns.
- The checker output of each file (copy the terminal text into a `.txt` file).
- A short note with anything unusual (robot stuck, grSim restarted, settings changed between recordings).

Put everything in one folder (e.g. `grsim_<date>`), zip it, and copy it to `nn_sim/data/grsim/` on this machine.

## 5. If the checker complains

| Message | What to do |
|---|---|
| `late ticks (> 25 ms)` above 2% | Close other programs. Keep the engine window open (not minimized). Re-record if it is above ~10%. |
| `robot not in vision` | Check that grSim vision is reaching the engine (the robot is drawn in the engine GUI) and that `robot_id`/`team` match a grSim robot. |
| `repeated vision frames` above 5% | grSim is sending vision slower than 60 Hz or dropping frames: check its vision settings and CPU load. |
| `jumps in the tick counter inside episodes` | The engine skipped ticks (overloaded PC). Same remedy as late ticks. |
| `[collect] waiting for robot 0 (team 0) in vision...` never ends | The robot is not visible: wrong id/team, SIM still on, or vision not configured. |

## Reference: what the script does

- **Episode:** the robot is teleported to a random pose near the center (|x| ≤ 1 m, |y| ≤ 0.7 m) and kept at rest for 0.5 s
  (`phase` 0), then driven by random commands (`phase` 1). The episode ends after 6 s (mix) or 40 s (waypoint), or as soon
  as the robot leaves the safety box |x| ≤ 3.6 m, |y| ≤ 2.4 m (the walls are at 4.8 m and 3.3 m).
- **Mix commands:** constant steps up to 1.3× the nominal limits (3.5 m/s, 10 rad/s), sums of sinusoids, filtered noise,
  chirps, and zero-command pauses, as in session 1.
- **Waypoint commands:** a go-to-point controller to random targets inside the safety box, slowing down on approach, never above the limits.
- **Settings:** the `CONFIG` table at the top of `collect_dynamics.lua`; the wrapper scripts override `mode`, `seed` and `duration_min`.
  Every setting is saved to the `.json` next to the CSV.
- **Dry run:** the same script was validated headless against the engine's built-in simulator. The logged (state, command) pairs reproduce
  the simulator's physics to 0.001 mm, and the checker recovers an artificial 3-tick command delay exactly.
