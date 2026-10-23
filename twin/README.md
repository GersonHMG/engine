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
python -m twin.prepare      # noise estimate, outliers, gaps, RTS smoother -> data/prepared.pkl
python -m twin.train        # baselines + GRU twin -> checkpoints/
python -m twin.evaluate     # -> reports/evaluation.md, reports/evaluation.json
```

For real robots, replace step 1 by logs from the engine's twin logger (plan
§2) in the same CSV format, without the `true_*` columns.

| file | plan | what |
|---|---|---|
| `robots.py` | — | reference robots (synthetic ground truth) |
| `excitation.py` | §2.2 | command programs: steps, chirp, PRBS, spin+translate, creep, creep_long, sprint, move_to, still; geofence |
| `generate.py` | §2 | sessions + SSL-Vision-like noise, written in the logger's CSV format |
| `data.py` | §3 | noise floor from `still`, outlier rejection, gap cutting, RTS smoother tuned by likelihood, robot-frame features |
| `models.py` | §4 | ssl-sim prior (+ delay), ARX, GRU residual twin; shared free-run `rollout` |
| `train.py` | §4.3 | delay search, baselines, free-run rollout loss on raw vision (Huber, in noise units), input-noise injection, horizon curriculum to 2 s |
| `simulate.py` | §5 | `Twin`: step-by-step simulator of one robot from a checkpoint |
| `replay.py` | §5 | session + the twin's free run of it as an engine replay CSV (blue = real, yellow = twin) |
| `evaluate.py` | §6 | open-loop rollouts, scenario maneuvers, 2-min free runs, closed loop with a controller, against fixed criteria |

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
