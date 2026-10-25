"""Numpy port of the simulator's robot update (ssl-sim `Robot::drive`).

Used only to check the data (frames, timing, clamps) and to produce exact
reference responses; it is never given to the models.

Per control tick of DT, `Simulator::step` runs ceil(DT / max_substep) equal substeps;
each one rotates the robot-frame command by the current θ, caps it at max_speed,
moves the velocity toward it by at most max_accel·h (same for ω with max_omega and
max_alpha), then integrates the pose with semi-implicit Euler.
"""

import math

import numpy as np

from .data import DT, wrap

DEFAULT = dict(max_speed=3.5, max_accel=6.0, max_omega=10.0, max_alpha=40.0, max_substep=0.001)


def clamp_length(x, y, limit):
    n = np.hypot(x, y)
    k = np.where(n > limit, limit / np.maximum(n, 1e-300), 1.0)
    return x * k, y * k


def step(s, u, p=DEFAULT, dt=DT):
    """One control tick for a batch of world states s (..., 6) and commands u (..., 3)."""
    x, y, th, vx, vy, w = (s[..., i].copy() for i in range(6))
    n = max(1, math.ceil(dt / p["max_substep"]))
    h = dt / n
    for _ in range(n):
        c, sn = np.cos(th), np.sin(th)
        tx, ty = clamp_length(c * u[..., 0] - sn * u[..., 1], sn * u[..., 0] + c * u[..., 1], p["max_speed"])
        dx, dy = clamp_length(tx - vx, ty - vy, p["max_accel"] * h)
        vx, vy = vx + dx, vy + dy
        tw = np.clip(u[..., 2], -p["max_omega"], p["max_omega"])
        w = w + np.clip(tw - w, -p["max_alpha"] * h, p["max_alpha"] * h)
        x, y = x + vx * h, y + vy * h
        th = wrap(th + w * h)
    return np.stack([x, y, th, vx, vy, w], axis=-1)


def rollout(s0, us, p=DEFAULT):
    """States after each command of us (T, 3) from s0 (6,): (T + 1, 6)."""
    out = [np.asarray(s0, float)]
    for u in us:
        out.append(step(out[-1], np.asarray(u, float), p))
    return np.stack(out)
