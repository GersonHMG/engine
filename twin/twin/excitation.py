"""Command programs for data collection (Python versions of lua/twin/*.lua).

Each program gives, per tick, a robot-frame command (vt, vn, omega) for a batch
of robots. Open-loop programs are generated up front; ``move_to`` and the
geofence need the robot's pose and are evaluated tick by tick.
"""

import numpy as np

from .robots import TICK

MAX_SPEED = 3.0  # m/s, translation commands are kept under this
MAX_OMEGA = 6.0  # rad/s
FENCE = (3.6, 2.4)  # |x|, |y| that trigger the return to the center, m

PROGRAMS = ("steps", "chirp", "prbs", "spin_translate", "spin_fast", "creep", "creep_long", "sprint", "move_to", "still")


def _segments(rng, ticks, hold, make):
    """Piecewise-constant commands: segments of `hold` (min, max) s, value from `make`."""
    out = np.zeros((ticks, 3))
    k = 0
    while k < ticks:
        n = int(rng.uniform(*hold) / TICK)
        out[k : k + n] = make()
        k += n
    return out


def open_loop(program: str, ticks: int, rng: np.random.Generator) -> np.ndarray:
    """(ticks, 3) commands, or zeros for closed-loop/still programs."""
    if program == "steps":
        state = {"on": False}

        def make():
            state["on"] = not state["on"]
            if not state["on"]:
                return (0.0, 0.0, 0.0)
            axis = rng.integers(3)
            sign = rng.choice([-1.0, 1.0])
            amp = rng.choice([0.25, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0]) if axis < 2 else rng.choice([1.0, 2.0, 4.0, 6.0])
            cmd = [0.0, 0.0, 0.0]
            cmd[axis] = sign * amp
            return tuple(cmd)

        return _segments(rng, ticks, (0.5, 1.5), make)

    if program == "chirp":
        t = np.arange(ticks) * TICK
        period = 20.0
        phase_t = t % period
        f0, f1 = 0.2, 5.0
        # Linear sweep within each period; the axis changes every period.
        phase = 2 * np.pi * (f0 * phase_t + (f1 - f0) * phase_t**2 / (2 * period))
        axis = (t // period).astype(int) % 4
        out = np.zeros((ticks, 3))
        amp = np.array([1.2, 1.2, 4.0])
        for a in range(3):
            out[axis == a, a] = amp[a] * np.sin(phase[axis == a])
        both = axis == 3  # forward and sideways together, out of phase
        out[both, 0] = amp[0] * np.sin(phase[both])
        out[both, 1] = amp[1] * np.cos(phase[both])
        return out

    if program == "prbs":
        def make():
            v = rng.uniform(-MAX_SPEED, MAX_SPEED, 2) * (rng.random(2) > 0.25)
            w = rng.uniform(-MAX_OMEGA, MAX_OMEGA) * (rng.random() > 0.3)
            return (v[0], v[1], w)

        return _segments(rng, ticks, (0.1, 1.0), make)

    if program == "spin_translate":
        def make():
            angle = rng.uniform(-np.pi, np.pi)
            speed = rng.uniform(0.5, 2.0)
            return (speed * np.cos(angle), speed * np.sin(angle), rng.choice([-1, 1]) * rng.uniform(1.0, 6.0))

        return _segments(rng, ticks, (2.0, 4.0), make)

    if program == "spin_fast":
        # Fast translation while spinning hard: wheel saturation (circles of
        # radius v/omega, under 1 m, so the robot stays on the field).
        def make():
            if rng.random() < 0.2:
                return (0.0, 0.0, 0.0)
            angle = rng.uniform(-np.pi, np.pi)
            speed = rng.uniform(2.0, MAX_SPEED)
            return (speed * np.cos(angle), speed * np.sin(angle), rng.choice([-1, 1]) * rng.uniform(3.0, MAX_OMEGA))

        return _segments(rng, ticks, (1.0, 2.5), make)

    if program == "creep":
        def make():
            return (rng.uniform(-0.3, 0.3), rng.uniform(-0.3, 0.3), rng.uniform(-1.0, 1.0) * (rng.random() > 0.5))

        return _segments(rng, ticks, (0.5, 2.0), make)

    if program == "creep_long":
        # Slow, long holds on one axis at a time: steady-state friction and dead-zone.
        def make():
            if rng.random() < 0.3:
                return (0.0, 0.0, 0.0)
            cmd = [0.0, 0.0, 0.0]
            axis = rng.integers(3)
            cmd[axis] = rng.choice([-1, 1]) * (rng.uniform(0.05, 0.4) if axis < 2 else rng.uniform(0.2, 1.5))
            return tuple(cmd)

        return _segments(rng, ticks, (1.5, 4.0), make)

    if program in ("move_to", "sprint", "still"):
        return np.zeros((ticks, 3))

    raise ValueError(f"unknown program {program!r}")


def to_robot_frame(world_v: np.ndarray, theta: np.ndarray) -> np.ndarray:
    c, s = np.cos(theta), np.sin(theta)
    return np.stack([c * world_v[:, 0] + s * world_v[:, 1], -s * world_v[:, 0] + c * world_v[:, 1]], axis=1)


def go_to(pose: np.ndarray, target: np.ndarray, max_speed: np.ndarray) -> np.ndarray:
    """Simple skill: P control on position and heading, capped speed."""
    error = target[:, :2] - pose[:, :2]
    dist = np.linalg.norm(error, axis=1, keepdims=True)
    speed = np.minimum(max_speed[:, None], 2.5 * dist)
    world_v = error / np.maximum(dist, 1e-9) * speed
    heading_error = np.angle(np.exp(1j * (target[:, 2] - pose[:, 2])))
    omega = np.clip(4.0 * heading_error, -MAX_OMEGA, MAX_OMEGA)
    return np.concatenate([to_robot_frame(world_v, pose[:, 2]), omega[:, None]], axis=1)


class Driver:
    """Gives each tick's commands for a batch running one program."""

    def __init__(self, program: str, n: int, ticks: int, rng: np.random.Generator):
        self.program = program
        self.rng = rng
        self.plan = np.stack([open_loop(program, ticks, rng) for _ in range(n)], axis=1)  # (ticks, n, 3)
        self.target = self._new_targets(n)
        self.max_speed = rng.uniform(1.5, MAX_SPEED, n)
        self.returning = np.zeros(n, dtype=bool)
        # sprint: 0 = going to the start line, 1 = running, 2 = braking.
        self.phase = np.zeros(n, dtype=int)
        self.timer = np.zeros(n, dtype=int)
        self.run_cmd = np.zeros((n, 3))
        self.start = self._sprint_starts(n)

    def _new_targets(self, n):
        return np.stack(
            [self.rng.uniform(-3.4, 3.4, n), self.rng.uniform(-2.2, 2.2, n), self.rng.uniform(-np.pi, np.pi, n)], axis=1
        )

    def _sprint_starts(self, n):
        """A spot at one end of the field and a heading, so that running along the
        long axis is a random body-frame direction (forward, sideways, diagonal...)."""
        side = self.rng.choice([-1.0, 1.0], n)
        return np.stack([side * 3.3, self.rng.uniform(-1.8, 1.8, n), self.rng.uniform(-np.pi, np.pi, n)], axis=1)

    def _sprint(self, pose):
        n = pose.shape[0]
        cmd = np.zeros((n, 3))
        going = self.phase == 0
        if going.any():
            cmd[going] = go_to(pose[going], self.start[going], np.full(going.sum(), 2.0))[:, :3]
            close = np.linalg.norm(pose[:, :2] - self.start[:, :2], axis=1) < 0.1
            aligned = np.abs(np.angle(np.exp(1j * (pose[:, 2] - self.start[:, 2])))) < 0.1
            ready = going & close & aligned
            if ready.any():
                # Constant body-frame command towards the other end, for the whole run.
                speed = self.rng.uniform(1.5, MAX_SPEED, n)
                direction = -np.sign(self.start[:, 0])
                world = np.stack([direction * speed, np.zeros(n)], axis=1)
                body = to_robot_frame(world, self.start[:, 2])
                self.run_cmd[ready] = np.concatenate([body, np.zeros((n, 1))], axis=1)[ready]
                self.phase[ready] = 1
        running = self.phase == 1
        cmd[running] = self.run_cmd[running]
        done = running & ((np.abs(pose[:, 0]) > 3.4) & (np.sign(pose[:, 0]) != np.sign(self.start[:, 0])) | (np.abs(pose[:, 1]) > 2.3))
        self.phase[done] = 2
        self.timer[done] = int(1.0 / TICK)
        braking = self.phase == 2
        self.timer[braking] -= 1
        finished = braking & (self.timer <= 0)
        if finished.any():
            self.start[finished] = self._sprint_starts(n)[finished]
            self.phase[finished] = 0
        return cmd

    def command(self, k: int, pose: np.ndarray) -> np.ndarray:
        n = pose.shape[0]
        if self.program == "sprint":
            cmd = self._sprint(pose)
            # The sprint keeps its own course; the geofence only guards a runaway.
            outside = (np.abs(pose[:, 0]) > 4.0) | (np.abs(pose[:, 1]) > 2.7)
            cmd[outside] = go_to(pose[outside], np.zeros((outside.sum(), 3)) + [0, 0, 0], np.full(outside.sum(), 1.5))
            self.phase[outside & (self.phase == 1)] = 2
            return self._limit(cmd)
        if self.program == "move_to":
            reached = np.linalg.norm(self.target[:, :2] - pose[:, :2], axis=1) < 0.05
            if reached.any():
                fresh = self._new_targets(n)
                self.target[reached] = fresh[reached]
                self.max_speed[reached] = self.rng.uniform(1.5, MAX_SPEED, reached.sum())
            cmd = go_to(pose, self.target, self.max_speed)
        else:
            cmd = self.plan[k].copy()

        # Geofence: outside it, drive back to the center (still useful data).
        outside = (np.abs(pose[:, 0]) > FENCE[0]) | (np.abs(pose[:, 1]) > FENCE[1])
        self.returning |= outside
        self.returning &= np.linalg.norm(pose[:, :2], axis=1) > 1.0
        if self.returning.any():
            home = np.concatenate([np.zeros((n, 2)), pose[:, 2:3]], axis=1)
            back = go_to(pose, home, np.full(n, 1.5))
            cmd[self.returning] = back[self.returning]

        return self._limit(cmd)

    @staticmethod
    def _limit(cmd):
        speed = np.linalg.norm(cmd[:, :2], axis=1, keepdims=True)
        cmd[:, :2] *= np.minimum(1.0, MAX_SPEED / np.maximum(speed, 1e-9))
        cmd[:, 2] = np.clip(cmd[:, 2], -MAX_OMEGA, MAX_OMEGA)
        return cmd
