"""Generate synthetic logs of the reference robots, with realistic vision noise.

Writes one CSV per session in the format of the planned engine twin logger,
plus ``true_*`` columns (noise-free) that only synthetic data has; the
pipeline never uses them for training, only to check the results.

    python -m twin.generate --out data/synthetic
"""

import argparse
import json
from pathlib import Path

import numpy as np

from .excitation import Driver
from .robots import SCENARIOS, TICK, RobotSim, quantize, scenario

SESSION_S = 120.0

# Sessions per robot: (split, program, count).
SESSIONS = [
    *[("train", p, 2) for p in ("steps", "chirp", "prbs", "spin_translate", "creep", "move_to")],
    *[("val", p, 1) for p in ("steps", "chirp", "prbs", "spin_translate", "creep", "move_to")],
    ("test", "steps", 1),
    ("test", "prbs", 1),
    ("test", "spin_translate", 1),
    ("test", "move_to", 1),
    ("noise", "still", 1),
]

# Added after the first evaluation showed gaps in coverage (slow steady driving,
# long fast straight runs). Train/val only, so the test set stays the same.
COVERAGE_SESSIONS = [
    ("train", "sprint", 2),
    ("train", "creep_long", 2),
    ("val", "sprint", 1),
    ("val", "creep_long", 1),
]

# Added after the third evaluation: fast driving while spinning hard (wheel
# saturation) was ~2 % of the data and where the twin was weakest.
SATURATION_SESSIONS = [
    ("train", "spin_fast", 2),
    ("val", "spin_fast", 1),
]

COLUMNS = (
    "tick,t_engine,id,team,cmd_vx,cmd_vy,cmd_omega,cmd_kick,cmd_dribbler,"
    "raw_x,raw_y,raw_theta,t_capture,camera_id,seen,"
    "true_x,true_y,true_theta,true_vt,true_vn,true_omega"
)


class Vision:
    """SSL-Vision-like measurements: delay, white noise, a calibration offset
    between the two cameras, outliers (incl. flipped heading) and dropouts."""

    def __init__(self, rng, delay=2, sigma_xy=0.0025, sigma_theta=0.01):
        self.rng = rng
        self.delay = delay
        self.sigma_xy = sigma_xy
        self.sigma_theta = sigma_theta
        self.camera_offset = np.array([0.004, -0.003, 0.004])  # camera 1 vs camera 0
        self.p_outlier = 0.002
        self.p_drop = 0.015
        self.p_burst = 0.002

    def measure(self, truth: np.ndarray):
        """truth: (T, 3) poses already delayed. Returns raw (T, 3), seen (T,), camera (T,)."""
        T = truth.shape[0]
        rng = self.rng
        camera = (truth[:, 0] >= 0).astype(int)
        raw = truth + camera[:, None] * self.camera_offset
        raw[:, :2] += rng.normal(0, self.sigma_xy, (T, 2))
        raw[:, 2] += rng.normal(0, self.sigma_theta, T)

        outlier = rng.random(T) < self.p_outlier
        jump = rng.uniform(0.05, 0.3, T)
        angle = rng.uniform(-np.pi, np.pi, T)
        flip = rng.random(T) < 0.3
        raw[outlier & ~flip, 0] += (jump * np.cos(angle))[outlier & ~flip]
        raw[outlier & ~flip, 1] += (jump * np.sin(angle))[outlier & ~flip]
        raw[outlier & flip, 2] += np.pi

        seen = rng.random(T) >= self.p_drop
        for start in np.flatnonzero(rng.random(T) < self.p_burst):
            seen[start : start + rng.integers(4, 20)] = False
        raw[:, 2] = np.angle(np.exp(1j * raw[:, 2]))
        return raw, seen, camera


def run_sessions(robot: str, program: str, n: int, rng: np.random.Generator):
    """Simulates n sessions of one program at once. Yields per-session arrays."""
    ticks = int(SESSION_S / TICK)
    sim = RobotSim(scenario(robot), n)
    start = np.stack([rng.uniform(-1, 1, n), rng.uniform(-1, 1, n), rng.uniform(-np.pi, np.pi, n)], axis=1)
    sim.reset(start)
    driver = Driver(program, n, ticks, rng)
    cmds = np.zeros((ticks, n, 3))
    poses = np.zeros((ticks, n, 3))
    body = np.zeros((ticks, n, 3))
    for k in range(ticks):
        # State at the start of tick k (what vision sees, before the delay).
        poses[k] = sim.pose
        body[k] = sim.body_velocity()
        cmds[k] = quantize(driver.command(k, sim.pose))
        sim.step(cmds[k])
    return cmds, poses, body


def write_session(path: Path, robot_id: int, cmds, poses, body, vision: Vision):
    T = cmds.shape[0]
    d = vision.delay
    # Vision reports the pose of `d` ticks ago.
    delayed_pose = np.concatenate([np.repeat(poses[:1], d, 0), poses[:-d]])
    delayed_body = np.concatenate([np.repeat(body[:1], d, 0), body[:-d]])
    raw, seen, camera = vision.measure(delayed_pose)
    t = np.arange(T) * TICK
    true_theta = np.angle(np.exp(1j * delayed_pose[:, 2]))
    table = np.column_stack(
        [
            np.arange(T), t, np.full(T, robot_id), np.zeros(T),
            cmds, np.zeros(T), np.zeros(T),
            np.where(seen[:, None], raw, np.nan), t - d * TICK, camera, seen,
            delayed_pose[:, :2], true_theta, delayed_body,
        ]
    )
    fmt = ["%d", "%.5f", "%d", "%d", "%.2f", "%.2f", "%.2f", "%d", "%d",
           "%.5f", "%.5f", "%.5f", "%.5f", "%d", "%d",
           "%.6f", "%.6f", "%.6f", "%.6f", "%.6f", "%.6f"]
    np.savetxt(path, table, delimiter=",", header=COLUMNS, comments="", fmt=fmt)


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--out", type=Path, default=Path("data/synthetic"))
    parser.add_argument("--seed", type=int, default=0)
    parser.add_argument("--coverage", action="store_true", help="only the coverage sessions (seed + 1)")
    parser.add_argument("--saturation", action="store_true", help="only the saturation sessions (seed + 2)")
    args = parser.parse_args()

    if args.saturation:
        rng, plan = np.random.default_rng(args.seed + 2), SATURATION_SESSIONS
    elif args.coverage:
        rng, plan = np.random.default_rng(args.seed + 1), COVERAGE_SESSIONS
    else:
        rng, plan = np.random.default_rng(args.seed), SESSIONS
    for robot_id, robot in enumerate(SCENARIOS):
        for split, program, count in plan:
            cmds, poses, body = run_sessions(robot, program, count, rng)
            for i in range(count):
                folder = args.out / robot / split
                folder.mkdir(parents=True, exist_ok=True)
                name = f"{program}_{i}"
                write_session(folder / f"{name}.csv", robot_id, cmds[:, i], poses[:, i], body[:, i], Vision(rng))
                meta = {"robot": robot, "id": robot_id, "team": 0, "program": program, "split": split, "synthetic": True}
                (folder / f"{name}.json").write_text(json.dumps(meta, indent=2))
            print(f"{robot:10s} {split:5s} {program:15s} x{count}")


if __name__ == "__main__":
    main()
