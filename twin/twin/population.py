"""A population of randomized robots for training a twin that generalizes
(docs/digital-twin-generalization.md §3.1).

Each robot gets its own hardware: any wheel can be weak, rollers can be
jammed, side friction, mass, delays, dead band, gyro loop and vision noise all
vary. The simulator also has effects the twin's physics prior cannot express
(traction limit, motor current lag, battery sag over a session), so the prior
is never the true model.

Held-out test robots (never in training): the three original scenarios,
fault combinations, a wheel that weakens during a session, and robots outside
the training ranges.

    python -m twin.population --out data/population
"""

import argparse
import json
from dataclasses import dataclass
from pathlib import Path

import numpy as np

from .excitation import Driver
from .generate import COLUMNS, Vision
from .robots import TICK, quantize

SUBSTEPS = 10
WHEEL_ANGLES = np.radians([60.0, 135.0, 225.0, 300.0])  # FL, BL, BR, FR
WHEEL_R = 0.08
G = 9.81

# Nominal hardware (same as robots.RobotParams).
NOMINAL = dict(
    mass=2.6, inertia=0.012, kp=30.0, feedforward=0.6, max_force=3.6, max_wheel_speed=4.0,
    force_deadband=0.25, roller_viscous=0.4, roller_coulomb=0.15, body_viscous=0.3, body_coulomb=0.4,
    lateral_coulomb=0.0, lateral_viscous=0.0, spin_viscous=0.002, gyro_gain=3.0, firmware_max_wheel_speed=3.5,
    command_delay=4, vision_delay=2,
    # Effects outside the twin's prior.
    traction_mu=0.7, motor_lag=0.01, battery_sag=0.0,
    noise_scale=1.0,
    wheel=(1.0, 1.0, 1.0, 1.0),  # per-wheel health: scales kp and max_force, and top speed
    degrade_wheel=-1, degrade_final=1.0,  # a wheel whose health falls to degrade_final over the session
)

# Hardware values the model may try to recover (privileged targets in training).
PRIVILEGED = ("wheel0", "wheel1", "wheel2", "wheel3", "log_roller", "lateral_coulomb", "lateral_viscous",
              "mass", "command_delay", "force_deadband", "gyro_gain", "traction_mu", "battery_sag", "motor_lag")


def privileged_vector(p: dict) -> list[float]:
    """Hardware as numbers of order 1, in PRIVILEGED order."""
    return [
        *p["wheel"],
        float(np.log(p["roller_viscous"] / NOMINAL["roller_viscous"])),
        p["lateral_coulomb"], p["lateral_viscous"] / 2.0,
        p["mass"] / NOMINAL["mass"], p["command_delay"] / 5.0, p["force_deadband"] * 2.0,
        p["gyro_gain"] / 3.0, p["traction_mu"], p["battery_sag"] * 5.0, p["motor_lag"] * 50.0,
    ]


def sample_robot(rng: np.random.Generator) -> dict:
    """One random robot. Most parts are healthy; faults are common enough to learn."""
    p = dict(NOMINAL)
    wheel = []
    for _ in range(4):
        wheel.append(float(rng.uniform(0.85, 1.15) if rng.random() < 0.7 else rng.uniform(0.2, 0.85)))
    p["wheel"] = tuple(wheel)
    jam = rng.uniform(0.5, 2.0) if rng.random() < 0.7 else rng.uniform(2.0, 12.0)
    p["roller_viscous"] = NOMINAL["roller_viscous"] * jam
    p["roller_coulomb"] = NOMINAL["roller_coulomb"] * np.sqrt(jam)
    side = rng.random() < 0.3
    p["lateral_coulomb"] = float(rng.uniform(0.3, 1.5) if side else rng.uniform(0.0, 0.2))
    p["lateral_viscous"] = float(rng.uniform(0.5, 3.0) if side else rng.uniform(0.0, 0.3))
    m = rng.uniform(0.75, 1.25)
    p["mass"], p["inertia"] = NOMINAL["mass"] * m, NOMINAL["inertia"] * m
    p["body_viscous"] = NOMINAL["body_viscous"] * rng.uniform(0.5, 2.0)
    p["body_coulomb"] = NOMINAL["body_coulomb"] * rng.uniform(0.5, 2.0)
    p["command_delay"] = int(rng.integers(2, 9))
    p["vision_delay"] = int(rng.integers(1, 4))
    p["force_deadband"] = float(rng.uniform(0.0, 0.5))
    p["gyro_gain"] = float(rng.uniform(0.0, 4.0))
    p["traction_mu"] = float(rng.uniform(0.4, 0.9))
    p["motor_lag"] = float(rng.uniform(0.005, 0.02))
    p["battery_sag"] = float(rng.uniform(0.0, 0.15))
    p["noise_scale"] = float(rng.uniform(0.5, 2.0))
    return p


def test_robots() -> dict:
    """Held-out robots, never in training."""
    lateral = dict(roller_viscous=1.0, roller_coulomb=0.2, lateral_coulomb=1.0, lateral_viscous=2.5)
    bad = dict(wheel=(0.4, 1.0, 1.0, 1.0))
    robots = {
        "perfect": {},
        "bad_wheel": bad,
        "lateral": lateral,
        "two_bad_wheels": dict(wheel=(0.4, 1.0, 0.5, 1.0)),
        "bad_wheel_lateral": {**bad, **lateral},
        "heavy_bad_wheel": dict(wheel=(1.0, 0.45, 1.0, 1.0), mass=3.3, inertia=0.015),
        "degrading_wheel": dict(degrade_wheel=3, degrade_final=0.3),
        "ood_dead_wheel": dict(wheel=(1.0, 1.0, 0.1, 1.0)),
        "ood_slow_radio": dict(command_delay=11),
    }
    return {name: {**NOMINAL, **changes} for name, changes in robots.items()}


@dataclass
class Batch:
    """Per-robot hardware as arrays, for vectorized simulation."""

    params: list

    def __post_init__(self):
        P = self.params
        col = lambda k: np.array([p[k] for p in P], dtype=float)
        self.n = len(P)
        for k in ("mass", "inertia", "kp", "feedforward", "max_force", "max_wheel_speed", "force_deadband",
                  "roller_viscous", "roller_coulomb", "body_viscous", "body_coulomb", "lateral_coulomb",
                  "lateral_viscous", "spin_viscous", "gyro_gain", "firmware_max_wheel_speed", "traction_mu",
                  "motor_lag", "battery_sag"):
            setattr(self, k, col(k))
        self.wheel = np.array([p["wheel"] for p in P], dtype=float)  # (n, 4)
        self.command_delay = np.array([p["command_delay"] for p in P], dtype=int)
        self.degrade_wheel = np.array([p["degrade_wheel"] for p in P], dtype=int)
        self.degrade_final = col("degrade_final")


class PopulationSim:
    """Like robots.RobotSim, with per-robot hardware and the extra effects."""

    def __init__(self, batch: Batch, duration: float):
        self.b = batch
        self.n = batch.n
        self.duration = duration
        self.drive = np.stack([-np.sin(WHEEL_ANGLES), np.cos(WHEEL_ANGLES)], axis=1)
        self.radial = np.stack([np.cos(WHEEL_ANGLES), np.sin(WHEEL_ANGLES)], axis=1)
        self.reset(np.zeros((self.n, 3)))

    def reset(self, pose):
        self.pose = np.array(pose, dtype=float)
        self.vel = np.zeros((self.n, 3))
        self.force = np.zeros((self.n, 4))
        self.time = 0.0
        self.max_delay = int(self.b.command_delay.max())
        self.queue = [np.zeros((self.n, 3)) for _ in range(self.max_delay)]

    def body_velocity(self):
        c, s = np.cos(self.pose[:, 2]), np.sin(self.pose[:, 2])
        vx, vy = self.vel[:, 0], self.vel[:, 1]
        return np.stack([c * vx + s * vy, -s * vx + c * vy, self.vel[:, 2]], axis=1)

    def health(self):
        """(n, 4) wheel health now, including a wheel degrading over the session."""
        h = self.b.wheel.copy()
        frac = min(self.time / self.duration, 1.0)
        rows = np.flatnonzero(self.b.degrade_wheel >= 0)
        if len(rows):
            w = self.b.degrade_wheel[rows]
            h[rows, w] *= 1.0 + (self.b.degrade_final[rows] - 1.0) * frac
        return h

    def step(self, command):
        self.queue.append(quantize(command))
        # Per-robot delay: robot i uses the command from command_delay[i] ticks ago.
        stacked = np.stack(self.queue[-self.max_delay - 1 :])  # (D+1, n, 3), oldest first
        active = stacked[-1 - self.b.command_delay, np.arange(self.n)]
        self.queue.pop(0)
        h = TICK / SUBSTEPS
        health = self.health()
        sag = 1.0 - self.b.battery_sag * min(self.time / self.duration, 1.0)
        for _ in range(SUBSTEPS):
            self._substep(active, h, health, sag)
        self.time += TICK

    def _substep(self, cmd, h, health, sag):
        b = self.b
        R = WHEEL_R
        vb = self.body_velocity()
        omega_target = cmd[:, 2:3] + b.gyro_gain[:, None] * (cmd[:, 2:3] - vb[:, 2:3])
        target = cmd[:, :2] @ self.drive.T + R * omega_target
        peak = np.abs(target).max(axis=1, keepdims=True)
        target *= np.minimum(1.0, b.firmware_max_wheel_speed[:, None] / np.maximum(peak, 1e-9))
        actual = vb[:, :2] @ self.drive.T + R * vb[:, 2:3]
        slip = vb[:, :2] @ self.radial.T

        kp = b.kp[:, None] * health
        wanted = b.feedforward[:, None] * target + kp * (target - actual)
        wanted = np.where(np.abs(wanted) < b.force_deadband[:, None], 0.0, wanted)
        top = b.max_wheel_speed[:, None] * (0.5 + 0.5 * health)
        limit = b.max_force[:, None] * health * sag[:, None] * np.clip(1.0 - np.abs(actual) / top, 0.0, 1.0)
        wanted = np.clip(wanted, -limit, limit)
        # Motor current lag: the force follows what the firmware asks, with a time constant.
        self.force += (wanted - self.force) * (1.0 - np.exp(-h / b.motor_lag[:, None]))
        force = self.force
        drive_f = force @ self.drive
        # Traction: the wheels cannot push harder than friction allows.
        grip = b.traction_mu * b.mass * G
        scale = np.minimum(1.0, grip / np.maximum(np.linalg.norm(drive_f, axis=1), 1e-9))
        drive_f *= scale[:, None]
        torque = R * force.sum(axis=1) * scale

        roller = -b.roller_viscous[:, None] * slip - b.roller_coulomb[:, None] * np.tanh(slip / 0.02)
        f_body = drive_f + roller @ self.radial
        speed = np.linalg.norm(vb[:, :2], axis=1, keepdims=True)
        direction = vb[:, :2] / np.maximum(speed, 1e-9)
        f_body -= b.body_viscous[:, None] * vb[:, :2] + b.body_coulomb[:, None] * direction * np.tanh(speed / 0.02)
        f_body[:, 1] -= b.lateral_viscous * vb[:, 1] + b.lateral_coulomb * np.tanh(vb[:, 1] / 0.02)
        torque -= b.spin_viscous * vb[:, 2] + 0.01 * np.tanh(vb[:, 2] / 0.05)

        acc_body = f_body / b.mass[:, None]
        alpha = torque / b.inertia
        c, s = np.cos(self.pose[:, 2]), np.sin(self.pose[:, 2])
        acc_world = np.stack([c * acc_body[:, 0] - s * acc_body[:, 1], s * acc_body[:, 0] + c * acc_body[:, 1]], 1)
        self.vel[:, :2] += acc_world * h
        self.vel[:, 2] += alpha * h
        self.pose += self.vel * h


MIX_PROGRAMS = ("steps", "chirp", "prbs", "spin_translate", "spin_fast", "creep_long", "sprint", "move_to")
STILL_S = 10.0  # every session starts at rest: measures the vision noise
SEGMENT_S = 15.0


class MixedDriver:
    """A session that switches program every SEGMENT_S, after STILL_S at rest.
    Each robot follows its own random sequence of programs."""

    def __init__(self, n, ticks, rng):
        self.drivers = [Driver(p, n, ticks, rng) for p in MIX_PROGRAMS]
        seg = int(SEGMENT_S / TICK)
        self.choice = rng.integers(len(MIX_PROGRAMS), size=(ticks // seg + 2, n))
        self.seg = seg
        self.still = int(STILL_S / TICK)

    def command(self, k, pose):
        outs = np.stack([d.command(k, pose) for d in self.drivers])  # (P, n, 3)
        if k < self.still:
            return np.zeros_like(outs[0])
        pick = self.choice[(k - self.still) // self.seg]
        return outs[pick, np.arange(pose.shape[0])]


def simulate(params: list, duration: float, rng):
    """Runs one session for each robot. Returns cmds, poses, body velocities (T, n, 3)."""
    ticks = int(duration / TICK)
    sim = PopulationSim(Batch(params), duration)
    n = len(params)
    sim.reset(np.stack([rng.uniform(-1, 1, n), rng.uniform(-1, 1, n), rng.uniform(-np.pi, np.pi, n)], axis=1))
    driver = MixedDriver(n, ticks, rng)
    cmds, poses, body = (np.zeros((ticks, n, 3)) for _ in range(3))
    for k in range(ticks):
        poses[k] = sim.pose
        body[k] = sim.body_velocity()
        cmds[k] = quantize(driver.command(k, sim.pose))
        sim.step(cmds[k])
    return cmds, poses, body


def write(path: Path, robot_id: int, name: str, params: dict, session: str, split: str, cmds, poses, body, rng):
    vision = Vision(rng, delay=params["vision_delay"], sigma_xy=0.0025 * params["noise_scale"],
                    sigma_theta=0.01 * params["noise_scale"])
    vision.p_outlier *= params["noise_scale"]
    vision.p_drop *= params["noise_scale"]
    d = vision.delay
    T = len(cmds)
    dpose = np.concatenate([np.repeat(poses[:1], d, 0), poses[:-d]])
    dbody = np.concatenate([np.repeat(body[:1], d, 0), body[:-d]])
    raw, seen, camera = vision.measure(dpose)
    t = np.arange(T) * TICK
    table = np.column_stack([
        np.arange(T), t, np.full(T, robot_id), np.zeros(T), cmds, np.zeros(T), np.zeros(T),
        np.where(seen[:, None], raw, np.nan), t - d * TICK, camera, seen,
        dpose[:, :2], np.angle(np.exp(1j * dpose[:, 2])), dbody,
    ])
    fmt = ["%d", "%.5f", "%d", "%d", "%.2f", "%.2f", "%.2f", "%d", "%d", "%.5f", "%.5f", "%.5f", "%.5f", "%d", "%d",
           "%.6f", "%.6f", "%.6f", "%.6f", "%.6f", "%.6f"]
    path.parent.mkdir(parents=True, exist_ok=True)
    np.savetxt(path, table, delimiter=",", header=COLUMNS, comments="", fmt=fmt)
    meta = {"robot": name, "id": robot_id, "team": 0, "program": session, "split": split, "synthetic": True,
            "params": {k: (list(v) if isinstance(v, tuple) else v) for k, v in params.items()},
            "privileged": privileged_vector(params)}
    path.with_suffix(".json").write_text(json.dumps(meta, indent=1))


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--out", type=Path, default=Path("data/population"))
    parser.add_argument("--train", type=int, default=400, help="training robots")
    parser.add_argument("--val", type=int, default=50, help="validation robots")
    parser.add_argument("--sessions", type=int, default=2, help="sessions per training robot")
    parser.add_argument("--seconds", type=float, default=100.0, help="per training session")
    parser.add_argument("--seed", type=int, default=10)
    args = parser.parse_args()
    rng = np.random.default_rng(args.seed)

    groups = {"train": [sample_robot(rng) for _ in range(args.train)], "val": [sample_robot(rng) for _ in range(args.val)]}
    for split, robots in groups.items():
        for s in range(args.sessions):
            cmds, poses, body = simulate(robots, args.seconds, rng)
            for i, p in enumerate(robots):
                name = f"{split}_{i:03d}"
                write(args.out / split / name / f"session{s}.csv", i, name, p, f"session{s}", split,
                      cmds[:, i], poses[:, i], body[:, i], rng)
            print(f"{split}: session {s} of {len(robots)} robots", flush=True)

    tests = test_robots()
    names = list(tests)
    params = [tests[n] for n in names]
    # Calibration drive (context) and two evaluation sessions per test robot.
    for session, seconds in (("calibration", 130.0), ("test0", 100.0), ("test1", 100.0)):
        if session == "calibration":
            # Calibrated while healthy; the wheel only weakens in the test sessions.
            run = [{**p, "degrade_wheel": -1} for p in params]
        else:
            run = params
        cmds, poses, body = simulate(run, seconds, rng)
        for i, name in enumerate(names):
            write(args.out / "test" / name / f"{session}.csv", 1000 + i, name, run[i], session, "test",
                  cmds[:, i], poses[:, i], body[:, i], rng)
        print(f"test: {session}", flush=True)


if __name__ == "__main__":
    main()
