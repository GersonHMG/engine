# Action scoring: the formula for each action

How every action in `lua/AI/actions/` is scored in one shared unit, the probability
that it ends in a goal for us. It applies the framework of *Action Scoring Framework
for a Goal-Scoring Agent* (Nov 2026) to the actions and calculators that exist today.
This file defines the formulas; it does not change code yet. Section 5 lists what
each action computes today and what changes.

## 1. The general formula

Every action `a` taken in state `s` is described by its outcomes, a list of
(probability, next state, time) triples, and scored as:

```latex
\text{score}(a, s) = \sum_{i} p_i \, \gamma^{t_i} \, V(s'_i) - \text{cost}(a)
```

- `p_i` are the outcome probabilities. They sum to 1: every action lists its failure
  outcomes too, not only success.
- `t_i` is the time, in seconds, until outcome `i` is reached. The discount is per
  second, not per step, because our actions take very different times (a shot
  leaves in 0.3 s, a dribble across the field takes 3 s). `γ ≈ 0.95` per second.
- `V(s')` is the value of the state the outcome leaves us in (section 2).
- `cost(a)` covers what the outcomes do not model (wear, a rule risk). It is 0 for
  every action below unless stated; hysteresis between actions stays in
  `Role.DELIBERATION_COST`, not in the score.

Each success probability is a product of stages, checkpoints in time that can each
be observed in the simulator (section 4):

```latex
P(\text{success}) = \prod_{k} P(\text{stage}_k \mid \text{stages}_{<k}) \cdot R_a
```

`R_a` is the per-action residual, 1 until episodes say otherwise. Every stage is
clamped to [0.01, 0.99].

Most stages are races: the ball (or our robot) against the fastest enemy. They all
use the existing race primitive, so they share one calibration (the profile's
`timing_noise`):

```latex
\text{race}(m) = \frac{1}{1 + e^{-m / \sigma}}, \qquad m = T_\text{enemy} - T_\text{us}
```

which is `profile.first_chance(margin)`. The margins come from `AI/calculator/tempo.lua`.

## 2. State values

Two grids over the field, one value per cell (0.25 m cells: 36 × 24 on the 9 × 6 m
division B field):

| Value | Meaning | Range |
|---|---|---|
| `V(c)` | we hold the ball in cell `c` | [0, 1] |
| `V_LOST(c)` | the enemy holds the ball in cell `c` | [−1, 0] |
| `V(goal)` | we scored | 1 |

`V_LOST` is not a constant: losing the ball in front of our own goal is much worse
than losing it in front of theirs. By symmetry it is our own grid seen from the
other side, scaled by how much we care about conceding:

```latex
V_\text{LOST}(c) = -\rho \, V(\text{mirror}(c)), \qquad \text{mirror}(x, y) = (-x, -y), \quad \rho \approx 0.5
```

`ρ = 0` gives the framework's `V_LOST = 0`; a larger `ρ` makes the team more careful
near its own goal. This is what replaces the hand-made "urgency" terms of
`kick_to_clear` and `dribble_to_clear` (section 5).

`V(c)` is computed offline by value iteration over the empty field (no robots) with
the actions of section 3 restricted to what does not need teammates or enemies
(kick to goal, dribble to a neighbouring cell), then corrected online (section 4).
Enemies and teammates enter through the stage probabilities at decision time, not
through `V`.

Note on angle-dependent states: `V` is indexed by ball position only. Whether the
robot is already lined up is not part of the state; it enters as time (`t_kick`)
and as the "kick released" stage.

## 3. The actions

Notation: `r` our robot, `b` the ball, `E` the active enemies, `K` the enemy
keeper (the enemy nearest their goal center, inside their defense area), `E∖K` the
other enemies, `G` the enemy goal center, `t_kick = tempo.line_up_time(r, b, target)`.

The first stage of every kicking action is the same: the kick has to happen before
an enemy reaches the ball.

```latex
P_\text{release}(t_\text{kick}) = \text{race}\big(\text{tempo.enemy\_time}(b, E) - t_\text{kick}\big)
```

If it fails, the enemy has the ball where it is: outcome `V_LOST(b)` at `t_kick`.

### 3.1 kick_to_goal (attacker, needs `can_kick`)

| Outcome | Probability | Next state | Time |
|---|---|---|---|
| goal | `P_rel · P_target · P_clear · P_keeper · R` | `V = 1` | `t_kick + flight(b, aim)` |
| missed, blocked, saved | the rest after release | `V_LOST(G)` (rebound, goal kick) | `t_kick + flight` |
| stolen before the kick | `1 − P_rel` | `V_LOST(b)` | `t_kick` |

Stages, each measured only on attempts that passed the previous one:

1. **released**: `P_rel(t_kick)` above.
2. **on target**: the kick's direction error `ε ~ N(0, σ_k)`, `σ_k = profile.kick.angle_error`,
   leaves the ball inside the posts. With `α` the half angle of the goal mouth seen from
   `b` and `δ` the angle between the aim and the goal center:
   `P_target = Φ((α − δ)/σ_k) − Φ((−α − δ)/σ_k)`
   (logistic approximation: `1/(1+e^{−1.7x})` for `Φ(x)`).
3. **not blocked**: no field enemy cuts the shot line before it reaches the goal:
   `P_clear = race(tempo.path_margin(b, aim, t_kick, E∖K, react))`.
4. **beats the keeper**: the keeper does not reach the aim point first:
   `P_keeper = race(react + tempo.enemy_time(aim, {K}) − (t_kick + flight(b, aim)))`.

The aim is the one that maximizes the product (today `tempo.best_aim` picks it by
margin alone). Splitting the keeper out of the blocking stage is what lets
episodes tell "blocked" from "saved".

### 3.2 pass (attacker, needs `can_kick`): to teammate `j` at point `p_j`

| Outcome | Probability | Next state | Time |
|---|---|---|---|
| received | `P_rel · P_lane · P_control · R` | `V(p_j)` | `t_kick + flight(b, p_j) + receive_time` |
| intercepted | `P_rel · (1 − P_lane)` | `V_LOST(cut point)` (≈ midpoint) | `t_kick + flight/2` |
| not controlled | `P_rel · P_lane · (1 − P_control)` | `V_LOST(p_j)` | `t_kick + flight` |
| stolen before the kick | `1 − P_rel` | `V_LOST(b)` | `t_kick` |

1. **released**: `P_rel(max(t_kick, t_ready_j))`.
2. **lane**: `P_lane = race(tempo.path_margin(b, p_j, t_kick, E_react, react))`, enemies
   moved with `tempo.enemies_at` while we line up.
3. **controlled**: the receiver traps or redirects it before an enemy reaches `p_j`:
   `P_control = race(react + tempo.enemy_time(p_j, E_react) − t_arrive − receive_time)`.

`score(pass) = max_j score(pass to j)`. The value of the receiver's next move (a
one-touch shot) is already in `V(p_j)`; a one-touch shot shows up as
`receive_time = 0`, so the pass is discounted less. `V(p_j)` uses the grid, so a
pass is never valued by "what the mate could do with an empty field" alone: at
decision time `V(p_j)` can be replaced by `score(kick_to_goal from p_j)` when that is
larger, which is how today's `tempo.pass_margin` chains the pass and the shot.

### 3.3 dribble (attacker, needs `has_ball`): to the neighbouring point `q`

The framework's "go to goal": carry the ball one step (`STEP ≈ 0.5 m`) in one of 8
directions (or towards the goal) with `move_ball`. Replaces `dribble_to_clear`,
which is one fixed direction of this action.

| Outcome | Probability | Next state | Time |
|---|---|---|---|
| kept | `P_keep · R` | `V(q)` | `t_dribble = STEP / v_dribble` |
| stolen | `1 − P_keep` | `V_LOST(q)` | `t_dribble` |

**kept**: no enemy gets its body on the ball along the way:
`P_keep = race(min over points of b→q of (tempo.enemy_time(point, E) − T_us(point)))`,
our robot moving at the dribbling speed (a profile value, slower than `cruise_speed`).

### 3.4 kick_to_clear (attacker, needs `can_kick`): into space at point `p`

| Outcome | Probability | Next state | Time |
|---|---|---|---|
| we get it first | `P_rel · P_in · race(m_loose)` | `V(p)` | `t_kick + flight(b, p)` |
| they get it first | `P_rel · P_in · (1 − race(m_loose))` | `V_LOST(p)` | same |
| out of the field | `P_rel · (1 − P_in)` | `V_LOST(exit point)` | same |
| stolen before the kick | `1 − P_rel` | `V_LOST(b)` | `t_kick` |

`m_loose = tempo.loose_ball_margin(r, b, p, allies, E)`; `P_in` is 1 when `p` is
`FIELD_MARGIN` inside the field, 0 otherwise (a kick error model can soften it).
`score = max_p` over the sampled directions. No urgency term: near our goal
`V_LOST(b)` is very negative, so a clear that only has to beat "stolen here" wins on
its own; far from our goal the same clear loses to a pass or a dribble.

### 3.5 go_to_ball (attacker without the ball)

| Outcome | Probability | Next state | Time |
|---|---|---|---|
| we reach it first | `race(m_ball)` | `V(b̂)` | `T_us` |
| they reach it first | `1 − race(m_ball)` | `V_LOST(b̂)` | `T_enemy` |

`b̂` the predicted ball (`ball_owner`'s lookahead), `m_ball = tempo.enemy_time(b̂, E) − T_us`,
`T_us = profile.time_to_reach(r, b̂)`. This is a value of the situation more than of
the choice (the attacker without the ball has nothing else to do), but scoring it
in the same unit makes it comparable across robots: role assignment can use it
(section 6).

### 3.6 receive_pass (supporter, while a pass is in flight to it)

Same outcomes as the receiving half of a pass: `P_control · V(p) + (1 − P_control) · V_LOST(p)`
with `p` the point on the ball's path the robot reaches. Single option while the
pass is in flight; its score is informative, not a choice.

### 3.7 position_for_pass (supporter): move to point `p`

The supporter's value is the pass it makes possible. Standing at `p` gives the
attacker an extra option worth `score(pass to p)` (3.2, with `t_ready = time_to_reach(r, p)`),
and it only matters when it beats what the attacker already has:

```latex
\text{score}(\text{go to } p) = \max\big(0,\ \text{score}(\text{pass to } p) - \text{score}^*_\text{attacker}\big)
```

`score*_attacker` is the attacker's best score without this supporter. The
supporter picks `p` maximizing this (today's grid of candidates). `W_SCORE`,
`W_LANE`, `W_TRAVEL`, `W_FAR` disappear: shot quality is `V(p)`, the lane is
`P_lane`, travel is `t_ready` (in `P_rel` and the discount), distance is in the
flight time.

### 3.8 defend and keep_goal (defender, goalkeeper)

Single-action roles, so the score does not choose anything. In the shared unit
their value is how much they reduce the enemy's best score: for a defender marking
`e`, `score = −Δ score_enemy(pass to e)`; for a guard or the keeper,
`score = −Δ score_enemy(kick_to_goal)`, computed with the enemy-side versions of
3.1 and 3.2. That gives a way to pick between mark targets and guard spots later;
until then they return a constant.

## 4. Correcting the formulas from episodes

Each attempt is labeled with the stage where it stopped, from simulator events:

| Action | Labels |
|---|---|
| kick_to_goal | stolen, wide, blocked, saved, goal |
| pass | stolen, intercepted, not_controlled, received |
| dribble | stolen, kept |
| kick_to_clear | stolen, out, lost, won |
| go_to_ball | lost, won |

Per stage and situation bucket (e.g. distance × angle cells), the hand formula is
blended with the counts, measured only on attempts that reached that stage:

```latex
P = \frac{k \, p_\text{hand} + \text{successes}}{k + \text{attempts}}, \qquad k \approx 20
```

`R_a` = observed successes / predicted successes, per action. `V` is corrected by
TD(0) after each decision: `V(s) ← V(s) + α (γ^{Δt} V(s') − V(s))`, `α ≈ 0.05`.

## 5. Today's scores and what changes

| Action | Score today | Score defined here |
|---|---|---|
| kick_to_goal | `can_kick · e^{−λd} · open_angle · clearance · alignment` | 3.1: 4 stages, keeper separate, time from `line_up_time` |
| pass | `can_kick · 0.9 · max_j lane · P_score(j) · alignment` | 3.2: release, lane, control; `V(p_j)` instead of `P_score` |
| kick_to_clear | `can_kick · lane · e^{−λ d(r, own goal)}` | 3.4: loose ball race, `V`/`V_LOST` of the landing point |
| dribble_to_clear | `has_ball · e^{−λ d(r, own goal)}` (unused) | 3.3: one direction of `dribble` (`push_ball` implements 3.3) |
| go_to_ball | `1 − can_kick` | 3.5 |
| receive_pass | `1` while receiving | 3.6 |
| position_for_pass | `0.5` (target chosen by `F(p)`) | 3.7 |
| defend, keep_goal | `1` | 3.8 (constant until enemy-side scores exist) |

Also removed once the scores share a unit:

- `Attacker.shoot_override` (`SHOOT_THRESHOLD`, `SHOOT_IN_RANGE`): it existed
  because shot and pass scores were not comparable. With 3.1 and 3.2 a ready shot
  beats a pass when its probability is higher than the discounted pass value.
- `KickAction.alignment` as a factor: being lined up is time (`t_kick`), which
  enters through `P_rel` and the discount.
- The four copies of `LAMBDA = 0.35` and the duplicated `probability_of_score` in
  `kick_to_goal.lua`: distance enters through `P_target` (the goal's angle shrinks)
  and through `V`.

## 6. Parameters

| Parameter | Value | Where |
|---|---|---|
| `γ` | 0.95 per second | new, `AI/calculator/value.lua` |
| `ρ` (`V_LOST` scale) | 0.5 | new |
| cell size | 0.25 m | new |
| dribble step, speed | 0.5 m, profile value | new, robot profile |
| `σ` (race width) | `profile.timing_noise` | exists |
| `σ_k` (kick direction error) | `profile.kick.angle_error` | exists |
| `k` (trust in the formula) | 20 attempts | new |
| `α` (TD rate) | 0.05 | new |

Open question for later: with every action in one unit, role assignment can become
"give the attacker role to the robot whose best score is highest" instead of the
time-to-ball rule in `ball_owner`.

## 7. What the simulator changed (implemented in `AI/calculator/value.lua`)

Measured with `lua/tests/attack.lua` (2v2 and 3v3 episodes, 100 tuning + 100 holdout
starts) and `lua/tests/shot_map.lua` (one shot per spot against the keeper skill).

| Formula part | Section 3 | Implemented | Why |
|---|---|---|---|
| P_keeper | race against the keeper's run to the aim | the keeper tracks the line ball -> goal center with a 0.4 s lag (`keeper_gap`), P = σ(gap / 4 cm) | keepers like `skills/goalkeeper` do not read the shot; the race predicted 0.02-0.6 where 94% scored |
| P_release | race to the ball | `1 − c·(1 − race)`, c = `profile.opponent.challenges_ball` | the test defenses reach the ball but never tackle (`stolen` 1-2%) |
| blocked shot, cut or uncontrolled pass, kick into space lost | V_LOST | `loose(p) = c·V_LOST(p) + (1 − c)·V(p)` | a ball an enemy only blocks stays ours unless it challenges |
| missed or saved shot | V_LOST(rebound) | V_LOST(aim) | the keeper holds the ball in its area, or it goes out |
| unavailable option | score 0 | −1 | 0 beat every option with a negative expected value: shots from out of range, waiting next to the ball |
| V grid | value iteration on the empty field | plus GRID_KEEP = 0.8 per kick into space, the model keeper, and a model guard when `opponent.guards_ball` | an empty field makes V flat (≈0.4 everywhere) and overrates passes against a ball guard |
| kicks into space | landing in the field | landing outside the areas and not rolling through the enemy's | a ball in the enemy area is the keeper's |
| execution | — | `kick_to_point` heading tolerance 0.1 → 0.015 rad | the ball leaves along the heading: 0.1 rad missed shots by 0.45 m (median) |
| P_clear, P_lane | race against every field enemy | an enemy body already on the path blocks it (−∞), no timing noise | the 3v3 ball guard, 0.35 m in front of the ball on the shot line, was raced at 39% clear and stopped every shot |
| kicks into a body | blocked outcome (`loose`) | not offered: no shot aim or clear direction whose path a robot already covers | no strategy may rely on bouncing the ball off an opponent (user rule) |
| P_keeper, long shots | race | × clamp((v_arrival − 0.3) / 0.8, 0, 1) | balls arriving under ~1 m/s get saved: 3.5-4 m 68%, > 4 m 17% vs P 0.88 / 0.74 (shot_map) |
| kick spot | outside the area by MARGIN | outside by the margin area_guard will keep there (more with a robot close) | a spot that fits only the smaller margin is never reached |
| blocked shot | V_LOST | `loose` 0.3 m past the ball, 0.6 s later | measured: shots into the guard moved the ball ~0.3 m per ~1 s |
| dribble (3.3) | `move_ball`, 8 directions | `push_ball`: 0.5 m pushes at 1 m/s, 0 / ±30° / ±60° off the goal line, also in the V grid; brakes when the ball rolling on (0.5 m) would reach an area | no dribbler, but a robot driving straight keeps a resting ball on its face up to 1.5 m/s |

Rule: only a goalkeeper may enter a defense area, and only its own (`utils/area_guard`
keeps every move target, straight path and braking point of our robots out of them).
