# Improving the attack strategy: the measurement loop

How a session improves the blue team's play (`lua/AI/`) by measuring it against fixed
yellow defenses in the headless simulator. This is the procedure that took the 2v2 attack
from 5% to 85-89% goals. Follow it step by step. The skill version is
`.claude/skills/improve-strategy/SKILL.md`.

Read these first:
- `simulator/CLAUDE.md`: headless runs, the `sim` table, and what the simulator does not
  model (no chip kick, no dribbler).
- `docs/action-scoring.md`: the score formula of every action, and section 7 on where the
  code departs from it.

---

## 0. Goal, rules and limits

- **Goal:** blue scores in at least 80% of episodes, on the tuning **and** the holdout
  starts, in **both** modes (2v2 and 3v3).
- **Game rule:** only the goalkeeper may enter its own goalkeeper (defense) area, never
  the opponent's. No other robot may enter either area. Touching an area with any part
  of the body counts as entering. `utils/area_guard.lua` enforces this for blue; do not
  weaken it.
- **Allowed:** change the Lua; add actions (`lua/AI/actions/`), skills (`lua/skills/`),
  calculators and tests; add scenarios (`scenarios/*.toml`).
- **Not allowed:**
  - changing the Rust engine;
  - changing the framework backbone drastically: `Role:choose` (options with
    termination and deliberation cost), the `Action` interface
    (`evaluate/start/run/terminated`), `role_assigner`, `ball_owner`, the tick loop in
    `AI/main.lua`;
  - changing the yellow defenses in `lua/tests/lib/defense_*.lua`. They are the fixed
    opponent the 80% is measured against.
- **Unit:** every score is "probability this action ends in a goal". Unavailable options
  score −1.0, not 0 (section 4, pitfall 1).

## 1. Setup (once per session)

```sh
# Linux container only, if cargo build fails on libudev / x11 (see README):
sudo apt-get install -y pkg-config libudev-dev libxkbcommon-dev libx11-xcb-dev libxcb1-dev libgl1-mesa-dev libwayland-dev libdbus-1-dev
cargo build --release             # ~7 min; Lua changes need no rebuild
SESSION=sessions/strategy-$(date +%F)  # git-ignored folder for everything the session produces
mkdir -p $SESSION/runs
```

Check the build: `./target/release/engine --headless lua/tests/kick_to_point.lua` must end
with `kicked=5/5`.

Speed: the machine has about 4 cores. One 100-episode 3v3 set takes 5–35 minutes,
depending on how much Lua the AI runs per tick. Run long jobs in the background and wait
on them; don't poll with sleeps.

## 2. The harness

`lua/tests/attack.lua` (environment variables; `attack_2v2.lua` is the 2v2 wrapper):

| Variable | Values |
|---|---|
| `MODE` | `2v2`: blue 0, 1 vs yellow marker (0) and keeper (1), the `run_ai.lua` defense. `3v3`: blue 0, 1, 2 (2 = blue keeper) vs marker, keeper and a ball guard (2), which stands 0.35 m in front of the ball |
| `SEEDS` | `tuning` (100 starts, for deciding), `holdout` (100, for checking), `named` (`scenarios/2v2_*`, `2vs1`, `marked_pass`, `3v3_*`) |
| `N` | number of episodes (use 10–30 for quick probes) |
| `RECORD` | JSON path: trajectories of the first 12 episodes |

- **Win:** the ball crosses x > 4.5 with |y| < 0.5 within 15 s.
- **Loss labels:**
  - `foul`: a blue body touches a forbidden area.
  - `out`, or `wide` after a shot.
  - `saved`: the yellow keeper holds the ball, or it lies dead in yellow's area.
  - `stolen`: a yellow field robot controls the ball.
  - `own_area`: a dead ball in our area.
  - `timeout`.
- **Each episode line:** the blue kicks with the action that caused each one, the ball's
  end position or the foul's location, and the scenario name.
- **SUMMARY line:** the win rate with its 95% Wilson interval, the label histogram, and
  `yellow_area_ticks`, which counts yellow breaking the rule (reported, not a loss).
- **Runs are deterministic:** the same code gives the same output, so any difference
  comes from your change.

Full measurement of all six sets in parallel:

```sh
lua/tests/run_attack.sh $SESSION NN-slug   # prints the 6 SUMMARY lines
```

With 100 episodes the interval is about ±9 points. A change of less than about 5 points
in one set is noise. Judge with both sets, and use 200 episodes when deciding between
close candidates.

## 3. The loop (one change per run)

1. **Baseline:** `lua/tests/run_attack.sh $SESSION 00-baseline`. Copy the current numbers
   into `$SESSION/report.md`.
2. **Find the biggest failure label** in the histogram of the weakest set. Then find
   *why* those episodes fail before changing anything (section 5). Write
   `$SESSION/runs/NN-slug/hypothesis.md`: the cause and its evidence, the one change, the
   predicted effect per set, and the risk.
3. **Make exactly one change.** Run the full measurement into `runs/NN-slug`, and save
   `git diff > runs/NN-slug/skill.diff`.
4. **Keep rule:** keep the change only if
   - the tuning rate rises in the mode you targeted,
   - no holdout rate drops by more than the noise (about 5 points),
   - there are no new fouls, and
   - `kick_to_point.lua` still kicks 5/5.

   Otherwise revert it (`git checkout <files>`) and keep the folder. Write
   `verdict.md`: prediction vs result, kept or reverted, why.
5. **Commit each kept change** on the session branch. The commit message carries the
   before → after numbers for all four sets. Push.
6. **Stop rule:** at least 80% on all four sets → final report. If two consecutive kept
   changes each add less than 3 points, or several runs in a row make a set worse, stop
   and report the label histogram and the best remaining idea. Don't keep grinding.

Stopping a run early is fine when its first 20–30 episodes are clearly bad. Record the
partial tallies in `verdict.md` and fold the fix into the next run.

## 4. Pitfalls already hit (check these first)

1. **Score 0 for "not possible" beats every negative option.** The attacker kicked
   hopeless shots, or waited next to the ball until the timeout. Unavailable → −1.0.
   `go_to_ball` is −1 when `can_kick`.
2. **Inconsistent failure terms.** If one action counts its failure outcomes (V_LOST,
   `loose`) and another doesn't, the one that doesn't always wins. Every kick counts
   success **and** failure.
3. **Attributing kicks.** A kick event belongs to the action chosen on the *previous*
   tick. The passer turns supporter the tick the ball leaves. `attack.lua` already
   handles this.
4. **A moving kick target never lines up.** With a 0.015 rad tolerance, `kick_to_point`
   at a moving teammate stays in `approach` forever. Commit the target point when the
   option starts.
5. **Dithering between options.** If two options are close and their scores depend on
   the robot's own heading, or on a teammate whose target depends on that heading, the
   attacker switches every few ticks and never kicks. Score support spots as if the
   passer already faced them.
6. **The path planner breaks the area rule.** `move_to` keeps only the robot's center out
   of the areas, and steers a robot that touches the ball out of the ball's obstacle,
   sometimes into the area. `area_guard` handles this: safe targets, `move_direct` near
   the areas, braking point, extra room when crowded. New skills need nothing extra,
   because the guard wraps the engine calls once `AI/main.lua` has run.
7. **Execution beats modelling.** The biggest single gain (2v2 27% → 80%) was the kick's
   heading tolerance (0.1 → 0.015 rad). Measure the skill before tuning probabilities.
8. **Validate the model against the real opponent.** Two examples:
   - The keeper doesn't react to shots. It tracks the ball → goal-center line with about
     0.4 s of lag, which `lua/tests/shot_map.lua` measured.
   - The test defenses never tackle (`profile.opponent.challenges_ball` = 0.1), so a
     blocked shot is usually still our ball (`value.loose`).

   Put what you scout about the opponent into `profile.opponent`, with a comment.
9. **The V grid is built without robots.** V(p) overrates holding the ball where the
   defense actually stands (for example a ball guard: `opponent.guards_ball`, set per
   mode by the harness). Pass values using V(p) inherit that.
10. **Test harness bugs look like strategy bugs.** Examples: a test's own skill driving
    into the area, `local` declared after use, a `nil` in the middle of an action list.
    Run a 2-episode smoke test after every harness edit.

## 5. Diagnosis tools

Use them in this order. Write scripts to `$SESSION` or the scratchpad, not the repo,
unless they are worth keeping.

1. **Label histogram + episode lines.** Which actions kicked, and where the ball ended
   (`grep -E "timeout|wide" <run>/<mode>-<set>.txt`).
2. **Action counts:**
   `grep -o "kicks=\[[^]]*\]" file | sed 's/kicks=\[//;s/\]//' | tr ',' '\n' | sort | uniq -c`.
   For example, 0 `kick_to_goal` in 200 episodes, or 0 `pass`.
3. **What happens after each action:** for every `pass`, what is the next kick or the
   episode end. This tells you whether the action pays off.
4. **Recorded frames** (`<mode>-<set>.json`). Each frame is
   `[t, ball x, ball y, x, y, θ for every robot in "robots" order, the action of each blue robot]`.
   For a kick, compare the aim with where the ball's line crosses x = 4.5. This found the
   0.45 m kick error.
5. **Score trace.** Override `draw_text` before `dofile("lua/tests/attack.lua")` and print
   the attacker's score labels and `kick_to_point.debug[team][id]` every 20 ticks
   (episode N: set `ATTACK_N`). This shows dithering and why a kick never fires.
6. **Command trace.** Wrap `move_to` / `move_direct` before the harness loads (the guard
   wraps on top of yours) to see the actual targets, and log the nearest robot. This
   found the planner steering into the area and the pushes.
7. **Calibration tests.** `lua/tests/shot_map.lua`: one shot per spot against the keeper,
   predicted P vs outcome per bucket. `OFFSET=1.0` starts the shooter 60° off the line;
   `KEEPER_LAG` / `KEEPER_SPEED` sweep the keeper model. Write similar ones for passes or
   receptions before trusting their formulas.
8. **Bisecting.** `git worktree add <scratch>/wtNN <commit>`, copy the current
   `lua/tests/attack.lua` into it, and run the same N starts in both trees at once.
   Remove the worktree afterwards.
9. **Ablations.** A temporary `os.getenv("NO_X")` switch around one component, run on 30
   episodes per variant (up to 3 in parallel). **Remove every switch before committing**
   (`grep -rn os.getenv lua/AI lua/utils lua/skills` must be empty).
10. **Timing.** Wrap `value.*` functions with `os.clock()` in a probe. Stay under about
    7 ms per tick, or runs get slow.

## 6. State at the end of the last session (session 2026-11-03, branch `ccr-d766ec81-ne6tee`, runs 00-13)

| Mode | Tuning | Holdout | Fouls | Main losses |
|---|---|---|---|---|
| 2v2 | 94% | 93% | 0 | saved 2–3, stolen 2, wide 1–2 |
| 3v3 | 82% | 80% | 0 | timeout 5–11, saved 6, stolen 0–4 |

- **No bounce strategies (user rule).** The attack must not shoot the ball into an opponent
  and play the rebound. `value.body_on_path`: shots and kicks into space whose path a robot
  body already covers are not offered. Check it with
  `sessions/.../probe/bounce.lua`-style counting (kicks whose ball next touches a yellow field
  robot): 3v3 39–44 per 100 episodes before, 8–11 after (robots moving into a kick).
- **Kick speed** in `profiles/sim_measured.lua` is the launch speed in play (3.48 m/s: the
  robot still moves ~0.55 m/s at the kick), and `P_keeper` has a slow-ball factor (shots
  arriving under ~1 m/s get saved). **Kick spots** must fit the margin `area_guard` will apply
  there (`value.guard_margin`: crowded next to the pinned guard), or the attacker circles the
  ball for ever.

- **What moved 3v3 from 57% to 77–80%:** `push_ball` (drive through the ball at 1 m/s: the
  simulator keeps it on the kicker face), so the attacker carries the ball up to where the
  area stops the ball guard instead of shooting into the guard about 5 times per episode.
  The model changes it needed: a robot body already on a path blocks it (`value.path_margin`
  returns −∞, no timing noise), the grid has push moves, the blocked shot is calibrated
  (0.3 m, 0.6 s), a push brakes when the ball rolling on would reach an area, and
  `kick_to_goal` re-aims once the ball moved more than 0.1 m since its aim was committed.
- **area_guard** also counts a robot that will be within `CROWD_DIST` in the next 0.2 s (the
  3v3 guard drives into ours at 2.8 m/s next to the area).
- **Tried and reverted:** kick_to_clear skipping directions into a standing robot (deflections
  off the guard are often a usable loose ball: 3v3 −5 / −13); `push_ball` backing off before
  turning to a new direction (removes a flick, but leaves more balls resting against the
  pinned guard: −4 / −1).
- **`pass` is still not offered** (`roles/attacker.lua`).
- **Ideas for the next session, most promising first:**
  1. **3v3 holdout timeouts (11).** Part of the pinned-guard deadlock is fixed (unreachable
     kick spots, run 13); trace the rest with a score/kick_to_point trace before changing
     anything. Balls at x ≈ 3.44–3.5 in front of the area cannot be kicked at all inside
     `area_guard.MARGIN`.
  2. **Shots from the area corner** (x ≥ 3.6, |y| ≥ 1) score below their prediction
     (65% vs 0.75 before the re-aim). Measure them with the guard near (shot_map variant).
  3. **The push flick:** turning to a new push direction with the ball on the face flicks it
     at 1.5 m/s. Keep the current direction unless the new one is clearly better.
  4. **Passing that pays** (as before).
- **Yellow breaks the area rule** (`yellow_area_ticks` > 0: `skills/mark` and the 3v3
  guard brush the edge). Ask the user before changing them, since that changes the
  benchmark.

## 7. Deliverables

- Kept changes committed and pushed on the session branch, with numbers in every commit
  message.
- `$SESSION/report.md` containing:
  - a table of all runs (kept, reverted or stopped),
  - the final numbers for all six sets,
  - the remaining failure labels,
  - the next idea.
- Optional: a replay page. Pass the 12 recorded holdout episodes of the baseline and the
  final runs to a page that draws the field (9 × 6 m, 1 × 2 m areas) and the robots
  from the frames.
