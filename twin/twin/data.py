"""Loading, cleaning and offline denoising of twin logs (plan §3).

Works on the engine twin-logger CSV format. ``true_*`` columns, when present
(synthetic data), are carried along for evaluation and never used to clean,
smooth or train.
"""

import json
import warnings
from dataclasses import dataclass, field
from pathlib import Path

import numpy as np

from .robots import TICK

MAX_GAP = 3  # ticks; longer runs of missing frames cut the sequence
MIN_SEGMENT = 90  # ticks; shorter clean pieces are dropped
OUTLIER_XY = 0.03  # m away from both neighbour estimates
OUTLIER_THETA = 0.25  # rad


@dataclass
class Session:
    path: Path
    robot: str
    robot_id: int
    program: str
    t: np.ndarray
    cmd: np.ndarray  # (T, 3) vt, vn, omega as sent
    raw: np.ndarray  # (T, 3) x, y, theta, NaN when not seen
    camera: np.ndarray
    truth: dict = field(default_factory=dict)  # synthetic only


def load_session(path: Path) -> Session:
    table = np.genfromtxt(path, delimiter=",", names=True)
    meta = json.loads(path.with_suffix(".json").read_text())
    seen = table["seen"].astype(bool)
    raw = np.column_stack([table["raw_x"], table["raw_y"], table["raw_theta"]])
    raw[~seen] = np.nan
    truth = {}
    if "true_x" in table.dtype.names:
        truth = {
            "pose": np.column_stack([table["true_x"], table["true_y"], np.unwrap(table["true_theta"])]),
            "body": np.column_stack([table["true_vt"], table["true_vn"], table["true_omega"]]),
        }
    return Session(
        path=path,
        robot=meta["robot"],
        robot_id=int(meta["id"]),
        program=meta["program"],
        t=table["t_engine"],
        cmd=np.column_stack([table["cmd_vx"], table["cmd_vy"], table["cmd_omega"]]),
        raw=raw,
        camera=table["camera_id"].astype(int),
        truth=truth,
    )


def load_split(root: Path, split: str, robots=None) -> list[Session]:
    sessions = []
    for robot_dir in sorted(p for p in root.iterdir() if p.is_dir()):
        if robots and robot_dir.name not in robots:
            continue
        for path in sorted((robot_dir / split).glob("*.csv")):
            sessions.append(load_session(path))
    return sessions


# --- noise ----------------------------------------------------------------


def wrap(a):
    return np.angle(np.exp(1j * a))


def measurement_noise(still: list[Session]) -> np.ndarray:
    """Std of x, y, theta from recordings of a robot at rest (robust: MAD)."""
    values = []
    for s in still:
        raw = s.raw[~np.isnan(s.raw[:, 0])]
        centre = np.array([np.median(raw[:, 0]), np.median(raw[:, 1]), np.angle(np.mean(np.exp(1j * raw[:, 2])))])
        dev = raw - centre
        dev[:, 2] = wrap(dev[:, 2])
        values.append(dev)
    dev = np.concatenate(values)
    return 1.4826 * np.median(np.abs(dev), axis=0)


# --- cleaning -------------------------------------------------------------


def reject_outliers(raw: np.ndarray) -> np.ndarray:
    """NaNs out detections far from what their neighbours predict.

    Two estimates per frame, from (k-1, k+1) and from (k-2, k+2); a frame is an
    outlier only if it disagrees with every estimate that exists, so a single
    bad neighbour cannot condemn a good frame.
    """
    raw = raw.copy()
    for _ in range(2):
        bad = np.zeros(len(raw), dtype=bool)
        dist_xy = np.full((len(raw), 2), np.nan)
        dist_th = np.full((len(raw), 2), np.nan)
        for j, step in enumerate((1, 2)):
            before = np.roll(raw, step, axis=0)
            after = np.roll(raw, -step, axis=0)
            before[:step] = np.nan
            after[-step:] = np.nan
            est_xy = (before[:, :2] + after[:, :2]) / 2
            est_th = np.angle(np.exp(1j * before[:, 2]) + np.exp(1j * after[:, 2]))
            dist_xy[:, j] = np.linalg.norm(raw[:, :2] - est_xy, axis=1)
            dist_th[:, j] = np.abs(wrap(raw[:, 2] - est_th))
        with warnings.catch_warnings():
            warnings.simplefilter("ignore", RuntimeWarning)  # all-NaN rows
            bad = (np.nanmin(dist_xy, axis=1) > OUTLIER_XY) | (np.nanmin(dist_th, axis=1) > OUTLIER_THETA)
        bad &= ~np.isnan(raw[:, 0])
        raw[bad] = np.nan
    return raw


def segments(seen: np.ndarray) -> list[tuple[int, int]]:
    """[start, end) ranges with no gap longer than MAX_GAP, trimmed to seen frames."""
    out = []
    idx = np.flatnonzero(seen)
    if len(idx) == 0:
        return out
    start = prev = idx[0]
    for k in idx[1:]:
        if k - prev - 1 > MAX_GAP:
            out.append((start, prev + 1))
            start = k
        prev = k
    out.append((start, prev + 1))
    return [(a, b) for a, b in out if b - a >= MIN_SEGMENT]


def unwrap_seen(theta: np.ndarray) -> np.ndarray:
    out = theta.copy()
    ok = ~np.isnan(theta)
    out[ok] = np.unwrap(theta[ok])
    return out


# --- RTS smoother ---------------------------------------------------------


def _ca_model(q: np.ndarray):
    """Constant-acceleration model, white jerk with intensity q per axis."""
    dt = TICK
    F = np.array([[1, dt, dt**2 / 2], [0, 1, dt], [0, 0, 1]])
    Q1 = np.array(
        [[dt**5 / 20, dt**4 / 8, dt**3 / 6], [dt**4 / 8, dt**3 / 3, dt**2 / 2], [dt**3 / 6, dt**2 / 2, dt]]
    )
    Q = q[:, None, None] * Q1[None]
    return F, Q


def rts_smooth(z: np.ndarray, r: np.ndarray, q: np.ndarray):
    """Forward Kalman filter + Rauch–Tung–Striebel backward pass.

    z: (T, A) measurements per axis (NaN = missing), r: (A,) measurement
    variance, q: (A,) jerk intensity. Returns mean (T, A, 3) [pos, vel, acc],
    variance of vel (T, A) and the innovation log-likelihood.
    """
    T, A = z.shape
    F, Q = _ca_model(q)
    x = np.zeros((A, 3))
    first = z[~np.isnan(z[:, 0])][0]
    x[:, 0] = first
    P = np.tile(np.diag([r.max() * 10 + 1e-6, 25.0, 400.0]), (A, 1, 1))
    xs_pred = np.zeros((T, A, 3))
    Ps_pred = np.zeros((T, A, 3, 3))
    xs = np.zeros((T, A, 3))
    Ps = np.zeros((T, A, 3, 3))
    loglik = 0.0
    for k in range(T):
        if k > 0:
            x = x @ F.T
            P = F @ P @ F.T + Q
        xs_pred[k], Ps_pred[k] = x, P
        if not np.isnan(z[k, 0]):
            S = P[:, 0, 0] + r
            innov = z[k] - x[:, 0]
            K = P[:, :, 0] / S[:, None]
            x = x + K * innov[:, None]
            P = P - K[:, :, None] * P[:, 0, None, :]
            loglik += -0.5 * np.sum(np.log(2 * np.pi * S) + innov**2 / S)
        xs[k], Ps[k] = x, P
    for k in range(T - 2, -1, -1):
        C = Ps[k] @ F.T @ np.linalg.inv(Ps_pred[k + 1])
        xs[k] = xs[k] + np.einsum("aij,aj->ai", C, xs[k + 1] - xs_pred[k + 1])
        Ps[k] = Ps[k] + C @ (Ps[k + 1] - Ps_pred[k + 1]) @ np.transpose(C, (0, 2, 1))
    return xs, Ps[:, :, 1, 1], loglik


@dataclass
class Segment:
    robot: str
    robot_id: int
    program: str
    cmd: np.ndarray  # (L, 3)
    raw: np.ndarray  # (L, 3) cleaned, theta unwrapped, NaN = missing
    pose: np.ndarray  # (L, 3) smoothed
    body: np.ndarray  # (L, 3) smoothed vt, vn, omega
    body_std: np.ndarray  # (L, 3)
    truth_pose: np.ndarray | None = None
    truth_body: np.ndarray | None = None


def to_body(world_v: np.ndarray, theta: np.ndarray) -> np.ndarray:
    c, s = np.cos(theta), np.sin(theta)
    return np.column_stack([c * world_v[:, 0] + s * world_v[:, 1], -s * world_v[:, 0] + c * world_v[:, 1]])


def prepare(session: Session, noise_std: np.ndarray, jerk: np.ndarray) -> list[Segment]:
    raw = reject_outliers(session.raw)
    out = []
    for a, b in segments(~np.isnan(raw[:, 0])):
        z = raw[a:b].copy()
        z[:, 2] = unwrap_seen(z[:, 2])
        xs, var_v, _ = rts_smooth(z, noise_std**2, jerk)
        pose = xs[:, :, 0]
        vt_vn = to_body(xs[:, :2, 1], pose[:, 2])
        body = np.column_stack([vt_vn, xs[:, 2, 1]])
        std_xy = np.sqrt(var_v[:, :2].mean(axis=1))
        seg = Segment(
            robot=session.robot,
            robot_id=session.robot_id,
            program=session.program,
            cmd=session.cmd[a:b],
            raw=z,
            pose=pose,
            body=body,
            body_std=np.column_stack([std_xy, std_xy, np.sqrt(var_v[:, 2])]),
        )
        if session.truth:
            truth_pose = session.truth["pose"][a:b].copy()
            # Same 2π branch as the measured heading.
            truth_pose[:, 2] += 2 * np.pi * np.round((pose[0, 2] - truth_pose[0, 2]) / (2 * np.pi))
            seg.truth_pose = truth_pose
            seg.truth_body = session.truth["body"][a:b]
        out.append(seg)
    return out


def fit_jerk(sessions: list[Session], noise_std: np.ndarray, grid=(3, 10, 30, 1e2, 3e2, 1e3, 3e3, 1e4, 3e4)) -> np.ndarray:
    """Jerk intensity per axis that maximizes the innovation likelihood (no truth needed)."""
    best = np.zeros(3)
    for axis in range(3):
        scores = []
        for q in grid:
            total = 0.0
            for s in sessions:
                raw = reject_outliers(s.raw)
                for a, b in segments(~np.isnan(raw[:, 0])):
                    z = raw[a:b, axis : axis + 1].copy()
                    if axis == 2:
                        z[:, 0] = unwrap_seen(z[:, 0])
                    total += rts_smooth(z, noise_std[axis : axis + 1] ** 2, np.array([q]))[2]
            scores.append(total)
        best[axis] = grid[int(np.argmax(scores))]
    return best
