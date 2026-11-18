# Controller `fault-aware` — 2026-11-18

Branch `controller/fault-aware`, controller `lua/tests/controller_bench/controllers/fault-aware.lua`.
Idea given with the skill: *design a controller that considers robot malfunctions such as
asymmetric bad wheels*.

## Scores

| | ideal | realistic | total | MLflow run |
|---|---|---|---|---|
| start (copy of `bang_bang`) | 36.29 | 152.57 | 188.87 | `43ab31f0112e4b1d9e6eb4b938b6b543` |
| final | 30.44 | 26.06 | 56.50 | `8de0ca7b876f477b8c24f24cf57c5977` |

The final controller is the new benchmark baseline (`results/baseline_*.csv`).

### Cost per robot × test (before → after)

| condition | robot | go_to_point | square | kick_to_point |
|---|---|---|---|---|
| ideal | perfect_robot | 1.69 → 1.98 | 5.82 → 6.45 | 1.21 → 1.14 |
| ideal | one_bad_wheel | 2.25 → 2.15 | 7.80 → 6.83 | 1.69 → 1.16 |
| ideal | bad_torque | 4.18 → 2.43 | 8.90 → 7.05 | 2.77 → 1.24 |
| realistic | perfect_robot | 2.96 → 1.60 | 8.72 → 4.95 | 5.90 → 0.96 |
| realistic | one_bad_wheel | 20 (NO) → 1.98 | 30 (NO) → 5.77 | 15 (no kick) → 0.95 |
| realistic | bad_torque | 20 (NO) → 2.37 | 35 (NO) → 6.45 | 15 (no kick) → 1.03 |

Every test now passes on every robot in both conditions. Under realistic conditions the weak robots
reach the pose (go_to_point), finish the square, and kick with a miss of 6–7 mm. With bang-bang they never did.
Kick miss under realistic conditions: perfect 327 → 13 mm, one_bad_wheel – → 7 mm, bad_torque – → 6 mm.

**One regression:** in `ideal`, the perfect robot is slower than with bang-bang (go_to_point +0.29 s,
square +0.63 s). The controller always assumes 66 ms of delay (the rules allow one constant).
With no delay, it predicts the robot further ahead than it is and brakes early.

## What the controller does

1. **Latency compensation.** It predicts the pose 66 ms ahead (one assumed constant, `LATENCY`) by
   replaying the commands it already sent. It controls that predicted pose, and turns the command into
   the robot frame with the heading predicted for when the command acts.
2. **Braking curve with a linear zone.** The wanted speed is `min(v_max, sqrt(2·a_brake·d), KP·d)`.
   The heading uses the same form.
3. **Command lead limit.** The command never runs more than 0.3 m/s (1.5 rad/s) ahead of the
   predicted velocity.
4. **Fault model, learned online.** It keeps the acceleration the robot actually reaches in each of
   16 directions of its own frame. When vision shows the robot clearly behind its command (saturated),
   it measures what the robot does and keeps that as the limit for that direction. A direction and its
   opposite share the limit, since motor torque is symmetric.
   It also probes upward: it ramps at 1.25 × the estimate, up to a cap of 5.5 m/s². When the robot
   keeps up, the estimate rises. The estimate sets the ramp, the braking curve (in the braking
   direction, robot frame) and the prediction model.
   Nothing about the robot is read: a weak wheel, low torque, or a healthy robot are all learned the
   same way.

## Attempts

| # | Hypothesis | ideal | realistic | total | MLflow | |
|---|---|---|---|---|---|---|
| 0 | start (bang-bang copy) | 36.29 | 152.57 | 188.87 | `43ab31f0` | — |
| 1 | latency prediction + linear zone + command lead limit | 30.01 | 29.25 | 59.26 | `ff6a0f18` | **kept** |
| 2 | online per-direction acceleration estimate, used for braking and prediction | 30.23 | 28.37 | 58.60 | `09caac93` | **kept** (seeds: 28.35 vs 29.19) |
| 3 | model the velocity lag in the robot frame inside the prediction | 30.24 | 28.37 | 58.60 | `bb3ecfc3` | rejected |
| 4 | share the estimate with the opposite direction | 30.78 | 27.75 | 58.54 | `da7245e1` | **kept** (seeds: 27.75 vs 28.35) |
| 5 | spread the turn over the drive | 31.52 | 29.65 | 61.18 | `e2e5bae9` | rejected |
| 6 | learn the acceleration only while turning slowly | 30.79 | 27.79 | 58.58 | `16ab4770` | rejected |
| 7 | probe the limit upward (ramp 1.25 × estimate, cap 4) | 30.49 | 26.29 | 56.78 | `b5f5d3fb` | **kept** (seeds: 26.34 vs 27.75) |
| 8 | acceleration cap 4 → 5.5 m/s² | 30.49 | 26.11 | 56.60 | `dc04ffde` | **kept** (seeds: 26.04 vs 26.34) |
| 9 | estimator gap = closest command over the delay window | 29.01 | 26.14 | 55.15 | `6c2ccefa` | rejected (seeds: 26.16 vs 26.04) |
| 10 | brake margin 0.85 → 0.93 | 29.89 | 26.09 | 55.98 | `4f1087e6` | rejected (judgment, see below) |
| 11 | 16 direction bins instead of 8 | 30.44 | 26.06 | 56.50 | `8de0ca7b` | **kept** (seeds: 25.98 vs 26.04) |
| 12 | v_max 2.5 → 3.0 | 30.46 | 26.02 | 56.48 | `f419b835` | rejected |

"Seeds" is the mean realistic score over the default noise sequence and `SIM_SEED=1, 2`, against the
best version so far.

The skill's stop rule (realistic below 30) was already met after #1. I kept going because #1 doesn't
contain the requested fault awareness: it is plain latency compensation. All the fault-model work is
in #2 onward, which stopped at the 12-attempt limit.

### Kept changes

- **#1 latency + linear zone + lead limit** (−129.6 total). With 66 ms of round-trip delay, bang-bang
  hunted around the pose and the weak robots went unstable. Predicting 66 ms ahead removes the
  hunting, the linear zone removes the infinite gain of the `sqrt` curve near the target, and the
  lead limit keeps a weak robot's command from running ahead of it. Realistic 152.57 → 29.25.
- **#2 per-direction acceleration estimate** (−0.66 total; realistic mean −0.84). The first step of
  the fault model. It learns forward ≈ 1.3–1.6 and sideways ≈ 2.0–2.5 m/s² on `bad_torque`.
- **#4 opposite directions share the estimate** (−0.06 total; realistic mean −0.60). On the square,
  `bad_torque` first brakes toward a direction it has never accelerated in, so it braked at 2.5 m/s² and overshot
  the first corner by 98 mm. With torque symmetry it brakes with the forward value it has already learned:
  go_to_point overshoot 115 → 2 mm, square 98 → 50 mm.
- **#7 upward probing** (−1.76 total; realistic mean −1.41). The fault model also raises the
  acceleration on a robot that can do more. The perfect robot's square 5.88 → 5.17 s; the weak robots
  saturate and keep their limits.
- **#8 cap 5.5 m/s²** (−0.18 total; realistic mean −0.30). The perfect robot can do 6 m/s².
- **#11 16 bins** (−0.10 total; realistic mean −0.06). This is within noise, but no robot gets worse, and a
  finer model fits an asymmetric wheel better. It can be dropped without loss.

### Rejected attempts

- **#3** Modelling the lag in the robot frame changed nothing. The path bend of the weak robots (dev 130–200 mm)
  builds up over the first 0.7 s, while the robot turns 90°. A 66 ms prediction can't see it.
- **#5** Spreading the turn over the drive gave 450 mm overshoot on the weak robots. The heading was still
  changing during braking, so the braking direction moved through bins the estimator hadn't learned.
- **#6** Learning only while turning slowly made no difference (27.79 against 27.75).
- **#9** Comparing with the closest command over the delay window lets the estimator learn without delay.
  Ideal improved 30.49 → 29.01, but realistic was 26.16 against 26.04 on the seed mean, so the rule rejects
  it. It is a good candidate if ideal matters.
- **#10** The 0.93 margin: the realistic seed mean was 25.98 against 26.04 (−0.2%, far inside noise).
  The gain all came from the perfect robot. One_bad_wheel (+0.34) and bad_torque (+0.20)
  got worse, with 38–47 mm overshoot, because their estimates aren't precise enough for a thin margin.
  That is the opposite of what this controller is for, so I rejected it.
- **#12** v_max 3.0 made no measurable difference (seed mean 25.96 against 25.98). I kept `v_max` equal to the baseline's.

## Robustness (realistic, three noise sequences)

| controller | default | seed 1 | seed 2 | mean ± half-range |
|---|---|---|---|---|
| bang_bang (start) | 152.57 | 151.84 | 145.94 | 150.1 ± 3.3 |
| fault-aware (final) | 26.06 | 25.89 | 26.00 | 25.98 ± 0.09 |

The final controller is better on every sequence, and much less sensitive to the noise.

## What still fails or costs

- **Ideal, perfect robot**: slower than bang-bang because of the fixed 66 ms latency assumption (see above).
- **Path bend on weak robots in go_to_point**: dev 132 mm (one_bad_wheel) and 205 mm (bad_torque).
  The cost doesn't count it, but on a field it would. It comes from turning 90° while the velocity
  still lags: the wheels track in the robot frame, so a lagging velocity turns with the body.
- **Forward estimate on `bad_torque` is low** (≈ 1.3–1.6 against ≈ 2.1 real). Attempt #6 didn't fix it.
  The cause is probably vision-velocity lag or the start of each saturation window.

## Next hypotheses

1. Path-following correction: steer the commanded velocity against the measured sideways velocity
   relative to the start→target line (cross-track feedback), not just aim from the current position.
   Aimed at the 130–200 mm bend.
2. Turn first, then drive, but only when the estimate shows a weak robot. That avoids #5's problem
   of braking while turning.
3. #9 (estimator over the whole delay window), combined with something that keeps realistic
   unchanged, for the ideal score.
4. A per-direction braking margin based on how many samples the bin has: thin where the estimate
   is well measured, wide where it is a guess. This is #10 without hurting the weak robots.
5. Learn the turn-rate limit online in the same way (the sim scales rotation with translation when
   a wheel saturates).
