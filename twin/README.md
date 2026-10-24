# twin — deep-learning digital twin of the robot's dynamics

Implements `docs/digital-twin-plan.md` §1–6: a model that takes the robot
commands the engine sends (vt, vn, ω, robot frame, quantized like the radio)
and predicts how the robot body moves. It is built to run **in a loop on its
own predictions**, like a simulator: training unrolls it on its own outputs
for up to 2 s, and evaluation free-runs it for whole 2-minute sessions and
inside a closed control loop.

There are no real robot logs yet, so the pipeline is developed and checked on
synthetic robots (`twin/robots.py`): a wheel-level model of a 4-wheel omni
robot with realistic vision noise, in three conditions:

| scenario | hardware |
|---|---|
| `perfect` | four healthy wheels |
| `bad_wheel` | weak front-left motor: curves and turns when told to go straight |
| `lateral` | jammed rollers and side friction: sideways motion slow, capped, dead-zone |

The model only ever sees noisy vision (2.5 mm / 0.6° noise, outliers, flipped
headings, dropouts, a camera offset, 2 ticks of vision delay); the noise-free
truth is used only to score it.

## Setup

```sh
cd twin
python -m venv .venv && . .venv/bin/activate
pip install -e .
```

## Pipeline

```sh
python -m twin.generate     # synthetic sessions -> data/synthetic/<robot>/<split>/*.csv (logger format)
python -m twin.generate --coverage   # + sprint / creep_long sessions (train, val)
python -m twin.generate --saturation # + spin_fast sessions (train, val)
python -m twin.prepare      # noise estimate, outliers, gaps, RTS smoother -> data/prepared.pkl
python -m twin.train        # baselines + GRU twin -> checkpoints/ (~1.5 h on 4 CPU cores)
python -m twin.evaluate     # -> reports/evaluation.md, reports/evaluation.json
```

For real robots, replace step 1 by logs from the engine's twin logger (plan
§2) in the same CSV format, without the `true_*` columns.

| file | plan | what |
|---|---|---|
| `robots.py` | — | reference robots (synthetic ground truth) |
| `excitation.py` | §2.2 | command programs: steps, chirp, PRBS, spin+translate, spin_fast, creep, creep_long, sprint, move_to, still; geofence |
| `generate.py` | §2 | sessions + SSL-Vision-like noise, written in the logger's CSV format |
| `data.py` | §3 | noise floor from `still`, outlier rejection, gap cutting, RTS smoother tuned by likelihood, robot-frame features |
| `models.py` | §4 | ssl-sim prior (+ delay), ARX, GRU residual twin; shared free-run `rollout` |
| `train.py` | §4.3 | delay search, baselines, free-run rollout loss on raw vision (Huber, in noise units), input-noise injection, recovery training, horizon curriculum to 2 s |
| `simulate.py` | §5 | `Twin`: step-by-step simulator of one robot from a checkpoint |
| `replay.py` | §5 | session + the twin's free run of it as an engine replay CSV (blue = real, yellow = twin) |
| `evaluate.py` | §6 | open-loop rollouts, scenario maneuvers, 2-min free runs, closed loop with a controller, against fixed criteria |

## Results (synthetic robots)

Full report: `reports/evaluation.md`. All errors are against the noise-free
truth on held-out sessions; the model only saw noisy vision.

1-s open-loop position error (median), GRU twin vs. the fitted ssl-sim model:

| robot | GRU twin | fitted ssl-sim | ARX |
|---|---|---|---|
| perfect | 1.6 cm | 14.2 cm | 22.0 cm |
| bad_wheel | 1.8 cm | 17.4 cm | 22.6 cm |
| lateral | 1.3 cm | 26.2 cm | 19.4 cm |

Vision noise is 2.5 mm per frame; one-step velocity error is ~1.3 cm/s.

| check (fixed before the results) | perfect | bad_wheel | lateral |
|---|---|---|---|
| 1 s median position ≤ 3 cm, heading ≤ 2° | pass | pass | pass |
| ≥ 50 % better than fitted ssl-sim | pass | pass | pass |
| 10 maneuvers from rest (2 s), each ≤ 5 cm and 3° | 7/10 | 8/10 | 10/10 |
| 2-min free runs: velocity RMSE ≤ 0.1 m/s | pass | pass | pass |
| 2-min free runs: error growth ≤ 1.5× | fail (2.4×) | fail (2.5×) | fail (2.1×) |
| closed loop, legs ≤ 2.5 m/s: paths ≤ 5 cm, settle ±0.15 s | pass | pass | pass |
| closed loop, legs at 3 m/s (paths; settle difference) | fail: 8.6–10.6 cm; ≤ 0.20 s | fail: 12.2–12.7 cm; ≤ 0.15 s | fail: 5.2–6.3 cm; ≤ 0.12 s |

Where it is still weak: driving above 2.5 m/s while spinning above 3 rad/s
(wheel saturation): 2.7–3.4 cm / 3–4° at 1 s, against 0.4–2.2 cm in every
other regime. The failing maneuvers are the 2–2.5 m/s straight runs and
1 m/s + 3 rad/s (5.5–7.4 cm, up to 5.5° after 2 s). The
error-growth ratio compares the first and last 30 s of a session, whose
commands differ, so it also fails when the absolute error is small (e.g.
0.016 m/s in `steps`). For scale on the 3 m/s legs: the real robot repeated
from starts 2.5 mm apart (what vision cannot resolve) spreads 1–2.7 cm, so
those 5–13 cm are model error, not chaos.

How the model got here (1-s error, perfect / bad_wheel / lateral):

| version | change | 1-s error |
|---|---|---|
| v1 | GRU residual, free-run training to 1 s | 2.4 / 2.3 / 2.0 cm |
| v2 | free-run to 2 s, input-noise injection | 2.1 / 2.3 / 1.8 cm |
| v3 | + `sprint`, `creep_long` data (coverage gaps) | 2.1 / 2.3 / 1.7 cm; 2.5 m/s straight 19 → 3 cm |
| v4 | + explicit 200 ms command window, 96 units | 1.5 / 1.8 / 1.3 cm |
| v5 | + recovery training (fine-tune) | 1.6 / 2.0 / 1.4 cm |
| v6 | retrained with all of it + `spin_fast` data | 1.6 / 1.8 / 1.3 cm; bad-wheel free-run lock-in gone |

Recovery training: in a quarter of the training windows the start velocity
is knocked off (0.3 m/s, 1.5 rad/s) and the model is scored on getting back
to the measured velocity. Without it, one out-of-distribution state fed back
into the model became a self-sustaining wrong spin in a 2-min free run.

## Using the twin as a simulator

```python
from twin.simulate import Twin

twin = Twin.load("checkpoints/twin_gru.pt", robot="bad_wheel")
twin.reset(0.0, 0.0, 0.0)               # at rest
for cmd in commands:                     # (vt, vn, omega), robot frame
    x, y, theta, vt, vn, omega = twin.step(cmd)
```

The state is what vision would report (including its delay). Runs are
deterministic.

## Context-adaptive twin (generalization to unseen robots)

Design: `docs/digital-twin-generalization.md`. No robot ID: an encoder reads a
recording of the robot driving and outputs a dynamics code `z`, which sets a
wheel-level physics prior and conditions (FiLM) the GRU correction. Trained on
400 randomized robots (`population.py`), tested on 9 robots never seen in
training. Training is teacher–student: the teacher computes `z` from the true
hardware (simulation only), and the context encoder (student) learns to
reproduce it from driving data.

```sh
python -m twin.population                                  # 400 + 50 random robots, 9 held-out test robots
python -m twin.train_adaptive prepare
python -m twin.train_adaptive train --phase teacher        # add --device cuda on a GPU
python -m twin.train_adaptive train --phase student --lr 1e-3
python -m twin.evaluate_adaptive                           # -> reports/evaluation_adaptive.md
```

Results (1-s open-loop error, cm, median; `reports/evaluation_adaptive.md`):

| held-out robot | no context | 30 s context | 30 s + test-time adaptation | true hardware (upper bound) | ssl-sim fitted to 120 s |
|---|---|---|---|---|---|
| perfect | 9.6 | 4.2 | 2.9 | 3.5 | 13.0 |
| bad_wheel | 11.9 | 3.9 | 2.0 | 2.5 | 13.9 |
| lateral | 4.4 | 2.7 | 2.7 | 1.6 | 15.8 |
| two_bad_wheels | 31.9 | 8.4 | 3.2 | 4.3 | 28.9 |
| bad_wheel_lateral | 25.6 | 3.1 | 2.8 | 2.4 | 22.5 |
| heavy_bad_wheel | 1.4 | 0.9 | 0.7 | 0.8 | 2.0 |

A wheel that weakens during a session: a rolling 30 s context gives 4.8 cm in
the second half, the stale healthy calibration 13.4 cm.

Not yet usable as a simulator. From rest (10 maneuvers, 6 closed-loop legs),
median errors are 4–10 cm (maneuvers) and 5–26 cm (closed loop) even with the
true-hardware `z`, and 7–24 / 8–54 cm with the estimated one. Both the
population dynamics model and the identification still need work; the v6 twin
above remains the better model for robots it was trained on.

Tried and dropped: fine-tuning the dynamics with the student's `z`
(`--phase finetune`) made held-out errors worse
(`reports/evaluation_adaptive_finetuned.md`). Training the teacher longer
gained little (validation 30.8 → 29.8).
