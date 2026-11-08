# Evaluation panel: is a Lua change better than the baseline?

Sidebar → flask icon (**Evaluation**). It plays every chosen scenario (`scenarios/*.toml`)
N times with your entry script (`lua/run_ai.lua` by default), once with the **working
tree's** Lua (the candidate) and once with the Lua of a **git revision** (the baseline,
`HEAD` by default), and tells you whether the candidate scores more often.

## How it runs

- One headless engine process per scenario and side (`engine --headless
  lua/tests/evaluate.lua`), as many at once as the machine has cores minus one. The GUI
  keeps running; **Cancel** kills the processes.
- The baseline's `lua/` is exported from git to `sessions/eval/baseline-<commit>/` and run
  with the working tree's harness, scenarios, `simulator.toml` and `robot_settings.toml`:
  only the Lua differs between the sides.
- Episode 1 starts exactly as the scenario was saved; episodes 2..N move every robot and the
  ball by up to *jitter* m (and headings by up to 2 × jitter rad). The jitter is seeded, so
  episode k of a scenario starts from the same place on both sides, and runs are
  reproducible (same code + settings = same numbers).
- Every episode reloads the entry script, so nothing carries over between episodes.
- Results of an episode: `goal`, `conceded`, `foul`, `out`, `cleared`, `saved`, `stolen`,
  `own_area`, `timeout` (definitions at the top of `lua/tests/evaluate.lua`).
- Baseline results are cached per commit and settings in `sessions/eval/cache/`, so
  re-evaluating after another Lua edit only plays the candidate. Every finished evaluation
  is saved as `sessions/eval/<unix time>.json`; the panel shows the last one when it opens.

## How to read it

- **Verdict:** exact McNemar test on the paired episodes (each episode played by both
  sides): only the episodes one side won and the other lost count. `BETTER` / `WORSE` when
  p < 0.05 over all episodes, otherwise `NO SIGNIFICANT DIFFERENCE`.
- **Δ win rate:** candidate − baseline, with a 95% bootstrap interval (episodes resampled
  within each scenario, 2000 resamples, seeded).
- **Table:** per scenario win rates, Δ with its interval, and the McNemar p value
  Holm-adjusted for the number of scenarios (green/red: significant after adjustment).
- **Plots** (`src/gui/plot`):
  - win rate per scenario, with Wilson 95% intervals;
  - Δ per scenario as a forest plot (diamond = all scenarios);
  - outcome shares per side;
  - time to goal as box plots;
  - the cumulative goal curve (share of episodes scored by time t).

How many episodes? With 10 per scenario a single scenario can only show big effects. All
scenarios together (e.g. 11 × 10 = 110 pairs) detect a change of roughly 10–15 points.
Multi-robot episodes are chaotic (tiny changes flip results), so trust the overall verdict
more than one scenario's row, and add episodes when the result is close.

## From a terminal

```sh
EVAL_SCENARIO=scenarios/2v2_wing.toml EVAL_N=5 ./target/release/engine --headless --max-time 200 lua/tests/evaluate.lua
cargo test --release -- --ignored evaluates_end_to_end   # the whole pipeline, 2 scenarios × 3 episodes
```
