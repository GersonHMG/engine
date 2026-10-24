# Design: a digital twin that generalizes to unseen robots

Follow-up to `digital-twin-plan.md`. The current twin (`twin/`, v6) tells the
network which robot it is simulating with a learned per-robot ID embedding.
That cannot work for:

- **a robot not in the training data** (new robot, spare, rebuilt robot);
- **a robot that changes** (a wheel wearing out mid-season, a new motor,
  battery level, a different carpet);
- **little data per robot**: every robot needs its own training data before
  it can be simulated at all.

The replacement removes the ID. The model **infers how a robot behaves from a
short recording of that robot driving** (its *context*), and is trained across
a large, varied population of robots so that this inference generalizes. This
combines four domain-adaptation techniques: domain randomization, context-based
adaptation (implicit system identification), test-time adaptation, and
sim-to-real feature alignment.

## 1. Idea

```
 context: a few seconds of this robot driving        what to simulate
 (commands + measured velocity)                       (commands)
              │                                            │
              ▼                                            │
     ┌──────────────────┐                                  │
     │ Context encoder  │──► z  (8-d "dynamics code",      │
     └──────────────────┘        mean + uncertainty)       │
                                  │                        │
                ┌─────────────────┼────────────────────┐   │
                ▼                 ▼                    │   │
     ┌──────────────────┐  ┌───────────────────────┐   │   │
     │ Physics params   │  │ GRU residual,         │◄──┴───┘
     │ head: z → wheel  │  │ conditioned on z      │
     │ gains, limits,   │  │ (FiLM)                │
     │ friction, delay  │  └──────────┬────────────┘
     └────────┬─────────┘             │
              ▼                       │
     ┌──────────────────┐             │
     │ Wheel-level      │── prior_v ──┤
     │ physics prior    │             ▼
     └──────────────────┘   next velocity = prior_v + correction
```

`z` replaces the robot ID: it is computed from data, not looked up. Two robots
that drive the same way get the same `z`, whatever their number. A robot whose
wheel weakens gets a `z` that drifts. An unseen robot gets a `z` as soon as it
has driven for a few seconds.

## 2. Architecture

### 2.1 Context encoder: q(z | context)

Input: a context window from the **same robot**, taken from a **different
time** than the stretch being predicted: 3–30 s of `(command history, measured
body velocity, velocity change)` per tick, from the smoothed data (plan §3).

- **Per-tick transition encoder**: MLP over each tick's local window (13
  commands, current and next velocity) → 64 features. Seeing one transition at
  a time makes the encoder about the robot's response, not about the
  particular trajectory.
- **Pooling over time**: attention-weighted mean over all ticks → order-free,
  works for any context length, and skips missing vision frames naturally.
- **Output**: mean and log-variance of `z ∈ R^8`. Short or uninformative
  contexts (a robot standing still says nothing about its wheels) give a wide
  variance; long, varied ones a narrow one.

### 2.2 Physics prior with parameters from z

The current prior (ssl-sim's accel-limited model) cannot express a weak wheel
or jammed rollers, so all of that falls on the network. Replace it with a
**differentiable wheel-level model** of the 4-wheel omni base:

- per wheel: motor gain, force limit, top wheel speed (12 values);
- roller friction, lateral friction, body friction, command delay (soft,
  as a learned mixture over 2–10 ticks), gyro loop gain.

A small head maps `z` to these ~18 parameters (bounded with sigmoids to physical
ranges). Faults then become interpretable parameter changes: a bad wheel is a
low gain on that wheel, not a pattern buried in a GRU. This structure is what
lets the model extrapolate to fault combinations it has not seen.

### 2.3 Residual network conditioned on z

Same GRU residual as now (2 × 96, 200 ms command window, velocity in →
correction out), but conditioned on `z` with **FiLM** (each GRU layer's
features scaled and shifted by a linear function of `z`) instead of an ID
embedding. It learns what the wheel model gets wrong: slip, motor inductance,
anything not in §2.2. Its correction stays small and bounded, so with an
unfamiliar `z` the twin falls back to the physics.

### 2.4 Output and integration

Unchanged: next body velocity `(vt, vn, ω)` → pose integrated outside the
network. Same `Twin.step(cmd)` interface, plus a `Twin.calibrate(recording)`
that sets `z`.

## 3. Domain-adaptation techniques

### 3.1 Domain randomization (sim pretraining)

Train on a **population of ~500 synthetic robots**, each with hardware drawn
from wide distributions:

| parameter | range |
|---|---|
| per-wheel motor gain, force limit, top speed | 0.2–1.2 × nominal, independently (covers one or several weak wheels) |
| roller friction, lateral friction | 0.5–10 × nominal (covers jammed rollers) |
| mass, inertia | ±25 % |
| command delay, vision delay | 2–8 ticks, 1–3 ticks |
| motor dead band, gyro gain | 0–0.5 N, 0–4 |
| vision noise, outlier and dropout rates | 0.5–2 × measured |

The reference simulator must also contain effects **outside** the prior's
structure (traction-limited wheel slip, battery sag over a session, motor
current lag, backlash). Otherwise the prior is the true model and the test is
rigged; real robots will have structure outside it too.

### 3.2 Context-based adaptation (implicit system identification)

Every training sample is a pair from the same robot: a context window and a
**separate** target window to predict. The encoder can only lower the loss by
putting into `z` what is constant about the robot. Losses:

- rollout loss on the target window, free-running on its own predictions for
  up to 2 s, with recovery samples (as in v6);
- `β · KL(q(z|context) ‖ N(0, I))`, small β: keeps `z` smooth and makes the
  variance meaningful;
- **privileged supervision in sim** (teacher–student, as in Rapid Motor
  Adaptation): a head predicts the true hardware parameters from `z`. This
  pushes `z` to encode physics, which transfers better to real robots than
  arbitrary features. Not available on real data, so it is an auxiliary loss
  only.

Random context lengths (0.5–30 s) in training teach the model to work with
whatever context exists.

### 3.3 Test-time adaptation

When a robot is far outside the training population (the encoder's `z` is
uncertain, or its predictions on the calibration data are poor), refine `z`
itself: freeze the network, run gradient descent on the 8 values of `z` to
minimize the rollout loss on the calibration recording. Few parameters, so a
minute of driving is enough and it cannot overfit badly. Optionally also adapt
the FiLM layers, with a penalty on moving away from the pretrained values.

### 3.4 Sim-to-real alignment

Real data differs from sim (lighting-dependent vision noise, unmodelled
mechanics). Fine-tune on real logs from several robots with the same losses,
minus the privileged one, mixing sim batches in so the sim knowledge is not
forgotten. If the encoder's features separate sim from real, add a
**domain-adversarial** term (gradient-reversal classifier on the transition
features) so they align. This is optional and only needed if validation on
real robots shows a gap; it can also erase real differences, so check it
against held-out real robots.

### 3.5 Online adaptation and change detection

In shadow mode (twin running next to a real robot), recompute `z` from a
rolling window of the last 30 s. A `z` that drifts beyond its own uncertainty
means the robot changed (wheel wearing out). The twin follows the change, and
the engine can warn about it. A robot's last `z` is saved in its profile
(`lua/profiles/`) for simulation without the robot.

### 3.6 Knowing when not to trust it

An ensemble of 3–5 models (different seeds) plus the `z` variance gives an
uncertainty per prediction. When it is high (an unseen regime or robot), the
twin warns and falls back towards the physics prior.

## 4. How it is used

1. **New robot:** drive the calibration script (`lua/twin/calibrate.lua`,
   about 60 s: steps on each axis, spin, combined motion) → `z` → saved in
   the robot's profile. No retraining.
2. **Simulation:** `Twin.from_profile(robot)` → simulate as now.
3. **Changed robot:** recalibrate, or let shadow mode update `z`.
4. **Refining for a robot:** test-time adaptation on its calibration data.

## 5. Evaluation: generalization, not memorization

The three current scenarios (perfect, bad wheel, lateral) are **held out of
training entirely**, so the model has never seen them. Test robots:

| test set | checks |
|---|---|
| the 3 scenarios, unseen | basic generalization |
| fault combinations: two weak wheels; weak wheel + jammed rollers; heavy robot with a weak wheel | extrapolation beyond single faults |
| a wheel weakening during a session | online adaptation, change detection |
| hardware outside the training ranges (gain 0.1, delay 10 ticks) | graceful degradation, uncertainty rises |

Metrics as in plan §6 (1-s and 2-s rollouts, maneuvers, 2-min free runs, closed
loop), reported **against the amount of context**: 0, 1, 5, 30, 120 s of the
robot's driving. That adaptation curve is the main result.

Comparisons:

| model | role |
|---|---|
| current v6 (robot ID), given the closest seen robot | what we have now |
| same network without ID or context | the "average robot" floor |
| fitted ssl-sim per test robot | physics-only, per-robot fit |
| v6-style model trained only on that robot's own data | per-robot upper bound (needs ~30 min of data) |

Target (to be confirmed after the first baseline numbers): with 30 s of
context, held-out robots within 25 % of the per-robot upper bound on 1-s
error, and better than the ID model on every unseen robot.

## 6. Implementation steps

1. Reference simulator: randomized hardware + effects outside the prior
   (§3.1). Generator for a robot population, with train/val/test robots.
2. Differentiable wheel-level prior (PyTorch), checked against the reference
   simulator with known parameters.
3. Context encoder, FiLM-conditioned GRU, paired context/target sampling,
   losses of §3.2.
4. Train on the population; evaluate on held-out robots against context length.
5. Test-time adaptation of `z`; ensemble uncertainty.
6. `Twin.calibrate` / `from_profile`; calibration Lua script; shadow mode
   (needs the engine logger from plan §2).
7. Real robots: fine-tune (§3.4) once logs from several robots exist.

## 7. Risks

- **The encoder learns the trajectory instead of the robot.** Mitigated by
  separate context and target windows, transition-level encoding and pooling.
- **The prior is too good on synthetic data**, which flatters the results.
  Mitigated by effects outside the prior in the reference simulator (§3.1),
  and finally only real robots can tell.
- **Sim population does not cover real robots.** Test-time adaptation and
  real fine-tuning are the fallback; the uncertainty in §3.6 flags it.
- **Compute:** ~500 robots × a few minutes each, a few hours of training on
  CPU per run.
