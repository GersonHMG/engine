"""Datasets produced by `engine-datagen`, and the robot-frame representation.

A dataset is a float64 array with one row per tick (columns in COLUMNS). Rows k and
k+1 of the same episode form the transition (s_k, u_k) -> s_{k+1}; the last row of
an episode has no command (NaN).

Representation (all in the robot frame at tick k, so it does not depend on x, y, θ):
    features z_k = (u_b, v_b, ω, vt, vn, ω_cmd)
    targets  Δ_k = (Δu_b, Δv_b, Δω, dx_b, dy_b, dθ)
where (u_b, v_b) is the world velocity rotated by -θ_k, Δu_b, Δv_b is the change of
velocity expressed in the same k frame, and (dx_b, dy_b, dθ) is the pose increment in
the k frame. `compose` turns a predicted Δ back into the next world state.
"""

import json
from dataclasses import dataclass
from pathlib import Path

import numpy as np

DATA_DIR = Path(__file__).resolve().parents[1] / "data"
DT = 1.0 / 60.0

COLUMNS = ["episode", "tick", "seg", "vt_cmd", "vn_cmd", "w_cmd", "x", "y", "theta", "vx", "vy", "omega"]
C = {name: i for i, name in enumerate(COLUMNS)}
SEGMENTS = ["step", "multisine", "noise", "chirp", "idle", "waypoint"]

FEATURES = ["u_b", "v_b", "omega", "vt_cmd", "vn_cmd", "w_cmd"]
TARGETS = ["du_b", "dv_b", "domega", "dx_b", "dy_b", "dtheta"]
# Physical units of the targets, for reports.
TARGET_UNITS = ["m/s", "m/s", "rad/s", "m", "m", "rad"]


def wrap(a):
    """Wraps angles to (-π, π]."""
    return np.pi - np.mod(np.pi - a, 2 * np.pi)


def rotate(x, y, angle):
    c, s = np.cos(angle), np.sin(angle)
    return c * x - s * y, s * x + c * y


@dataclass
class Dataset:
    name: str
    rows: np.ndarray  # (n_rows, len(COLUMNS))
    meta: dict

    @property
    def n_episodes(self):
        return int(self.meta["episodes"])

    @property
    def ticks(self):
        """Transitions per episode."""
        return int(self.meta["ticks_per_episode"])

    def episodes(self):
        """Rows reshaped to (episodes, ticks + 1, columns)."""
        return self.rows.reshape(self.n_episodes, self.ticks + 1, len(COLUMNS))

    def subset(self, n_episodes):
        meta = dict(self.meta, episodes=n_episodes)
        return Dataset(self.name, self.rows[: n_episodes * (self.ticks + 1)], meta)


def load(name, data_dir=DATA_DIR):
    rows = np.load(Path(data_dir) / f"{name}.npy")
    meta = json.loads((Path(data_dir) / f"{name}.json").read_text())
    return Dataset(name, rows, meta)


def states(ep):
    """World state (x, y, θ, vx, vy, ω) of episode array(s) (..., T, columns)."""
    return ep[..., [C["x"], C["y"], C["theta"], C["vx"], C["vy"], C["omega"]]]


def commands(ep):
    return ep[..., [C["vt_cmd"], C["vn_cmd"], C["w_cmd"]]]


def features(s, u):
    """z from world state s (..., 6) and command u (..., 3)."""
    ub, vb = rotate(s[..., 3], s[..., 4], -s[..., 2])
    return np.stack([ub, vb, s[..., 5], u[..., 0], u[..., 1], u[..., 2]], axis=-1)


def targets(s0, s1):
    """Δ between consecutive world states, in the frame of s0."""
    th = s0[..., 2]
    du, dv = rotate(s1[..., 3] - s0[..., 3], s1[..., 4] - s0[..., 4], -th)
    dx, dy = rotate(s1[..., 0] - s0[..., 0], s1[..., 1] - s0[..., 1], -th)
    return np.stack([du, dv, s1[..., 5] - s0[..., 5], dx, dy, wrap(s1[..., 2] - s0[..., 2])], axis=-1)


def compose(s0, delta, xp=np):
    """Next world state from s0 and a Δ in the frame of s0 (numpy or torch)."""
    th = s0[..., 2]
    c, s = xp.cos(th), xp.sin(th)
    vx = s0[..., 3] + c * delta[..., 0] - s * delta[..., 1]
    vy = s0[..., 4] + s * delta[..., 0] + c * delta[..., 1]
    x = s0[..., 0] + c * delta[..., 3] - s * delta[..., 4]
    y = s0[..., 1] + s * delta[..., 3] + c * delta[..., 4]
    theta = th + delta[..., 5]  # unwrapped during rollouts; wrap when comparing
    omega = s0[..., 5] + delta[..., 2]
    return xp.stack([x, y, theta, vx, vy, omega], axis=-1)


def transitions(ds):
    """Flattened (z, Δ, s0, s1, u, seg) for every transition of a dataset."""
    ep = ds.episodes()
    s, u = states(ep), commands(ep)
    s0, s1, u0 = s[:, :-1], s[:, 1:], u[:, :-1]
    seg = ep[:, :-1, C["seg"]].astype(int)
    flat = lambda a: a.reshape(-1, a.shape[-1]) if a.ndim == 3 else a.reshape(-1)
    return dict(z=flat(features(s0, u0)), d=flat(targets(s0, s1)), s0=flat(s0), s1=flat(s1), u=flat(u0), seg=flat(seg))
