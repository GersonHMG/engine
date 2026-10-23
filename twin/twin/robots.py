"""Reference robots: stand-ins for the real robot until real logs exist.

A wheel-level model of a 4-wheel omni SSL robot, simulated at 1 kHz. Each
wheel's motor pushes along the wheel's drive direction to track the wheel
speed the firmware asks for (inverse kinematics of the command); the body
moves under the sum of the wheel forces, roller friction and body friction.
This gives the effects the twin has to learn (latency, motor lag, torque and
speed limits, dead-zone, coupling) without writing any of them as such.

The scenarios change hardware parameters only:

- ``perfect``: four identical healthy wheels.
- ``bad_wheel``: the front-left motor is weak (low gain, low torque, low top
  speed), so the robot curves and turns when told to go straight.
- ``lateral``: jammed omni rollers and extra side friction, so sideways
  motion is sluggish, slower and has a large dead-zone.

The simulation runs many independent robots at once (leading batch axis).
"""

from dataclasses import dataclass, field, replace

import numpy as np

TICK = 1.0 / 60.0  # engine control period, s
SUBSTEPS = 16  # physics steps per tick (~1 kHz)


@dataclass(frozen=True)
class RobotParams:
    mass: float = 2.6  # kg
    inertia: float = 0.012  # kg m^2
    wheel_radius_pos: float = 0.08  # distance from center to wheel contact, m
    # Angle of each wheel's position, rad, 0 = front (+x). FL, BL, BR, FR.
    wheel_angles: tuple = (np.radians(60), np.radians(135), np.radians(225), np.radians(300))
    # Motor and firmware, per wheel.
    kp: tuple = (30.0, 30.0, 30.0, 30.0)  # N per (m/s) of wheel speed error
    feedforward: tuple = (0.6, 0.6, 0.6, 0.6)  # N per (m/s) of target wheel speed
    max_force: tuple = (3.6, 3.6, 3.6, 3.6)  # N at stall
    max_wheel_speed: tuple = (4.0, 4.0, 4.0, 4.0)  # m/s where torque reaches 0
    force_deadband: float = 0.25  # N: smaller motor commands do nothing (PWM dead band)
    # Rollers (perpendicular to the drive direction) and body.
    roller_viscous: float = 0.4  # N per (m/s) of roller slip, per wheel
    roller_coulomb: float = 0.15  # N per wheel
    body_viscous: float = 0.3  # N per (m/s)
    body_coulomb: float = 0.4  # N
    lateral_coulomb: float = 0.0  # extra side-only friction (robot frame), N
    lateral_viscous: float = 0.0  # extra side-only friction, N per (m/s)
    spin_viscous: float = 0.002  # N m per (rad/s)
    # Firmware yaw-rate loop on the gyro: extra wheel speed per (rad/s) of
    # spin error, so a weak wheel does not make the robot spin away.
    gyro_gain: float = 3.0
    # Firmware scales all wheel targets down together so none exceeds this
    # (keeps the direction of motion when the command is too fast).
    firmware_max_wheel_speed: float = 3.5
    command_delay: int = 4  # ticks from command to the motors
    max_command: float = 5.11  # radio clamp


def scenario(name: str) -> RobotParams:
    base = RobotParams()
    if name == "perfect":
        return base
    if name == "bad_wheel":
        return replace(
            base,
            kp=(12.0, 30.0, 30.0, 30.0),
            max_force=(1.8, 3.6, 3.6, 3.6),
            max_wheel_speed=(3.6, 4.0, 4.0, 4.0),
        )
    if name == "lateral":
        return replace(
            base,
            roller_viscous=1.0,
            roller_coulomb=0.2,
            lateral_coulomb=1.0,
            lateral_viscous=2.5,
        )
    raise ValueError(f"unknown scenario {name!r}")


SCENARIOS = ("perfect", "bad_wheel", "lateral")


def quantize(command: np.ndarray, max_command: float = 5.11) -> np.ndarray:
    """The radio's ×100 → integer → clamp path (packet_serializer.rs)."""
    q = np.trunc(np.asarray(command, dtype=np.float64) * 100.0)
    return np.clip(q, -max_command * 100, max_command * 100) / 100.0


@dataclass
class RobotSim:
    """A batch of independent robots with the same hardware."""

    params: RobotParams
    n: int
    pose: np.ndarray = field(init=False)  # (n, 3): x, y, theta (world)
    vel: np.ndarray = field(init=False)  # (n, 3): vx, vy (world), omega
    queue: list = field(init=False)  # delayed commands

    def __post_init__(self):
        p = self.params
        angles = np.asarray(p.wheel_angles)
        # Body-frame unit vectors per wheel.
        self.drive = np.stack([-np.sin(angles), np.cos(angles)], axis=1)  # (4, 2)
        self.radial = np.stack([np.cos(angles), np.sin(angles)], axis=1)  # (4, 2)
        self.kp = np.asarray(p.kp)
        self.ff = np.asarray(p.feedforward)
        self.fmax = np.asarray(p.max_force)
        self.smax = np.asarray(p.max_wheel_speed)
        self.reset(np.zeros((self.n, 3)))

    def reset(self, pose: np.ndarray):
        self.pose = np.array(pose, dtype=np.float64)
        self.vel = np.zeros((self.n, 3))
        self.queue = [np.zeros((self.n, 3)) for _ in range(self.params.command_delay)]

    def body_velocity(self) -> np.ndarray:
        """(n, 3): forward, leftward, omega."""
        c, s = np.cos(self.pose[:, 2]), np.sin(self.pose[:, 2])
        vx, vy = self.vel[:, 0], self.vel[:, 1]
        return np.stack([c * vx + s * vy, -s * vx + c * vy, self.vel[:, 2]], axis=1)

    def step(self, command: np.ndarray):
        """One engine tick with a (n, 3) robot-frame command (vt, vn, omega)."""
        self.queue.append(quantize(command, self.params.max_command))
        active = self.queue.pop(0)
        h = TICK / SUBSTEPS
        for _ in range(SUBSTEPS):
            self._substep(active, h)

    def _substep(self, cmd: np.ndarray, h: float):
        p = self.params
        R = p.wheel_radius_pos
        vb = self.body_velocity()  # (n, 3)

        # Wheel speeds: asked by the firmware, and actual at the contact.
        omega_target = cmd[:, 2:3] + p.gyro_gain * (cmd[:, 2:3] - vb[:, 2:3])
        target = cmd[:, :2] @ self.drive.T + R * omega_target  # (n, 4)
        peak = np.abs(target).max(axis=1, keepdims=True)
        target *= np.minimum(1.0, p.firmware_max_wheel_speed / np.maximum(peak, 1e-9))
        actual = vb[:, :2] @ self.drive.T + R * vb[:, 2:3]
        slip = vb[:, :2] @ self.radial.T  # roller direction, (n, 4)

        force = self.ff * target + self.kp * (target - actual)
        force = np.where(np.abs(force) < p.force_deadband, 0.0, force)
        # Torque falls with wheel speed (back-EMF) and is capped.
        limit = self.fmax * np.clip(1.0 - np.abs(actual) / self.smax, 0.0, 1.0)
        force = np.clip(force, -limit, limit)
        roller = -p.roller_viscous * slip - p.roller_coulomb * np.tanh(slip / 0.02)

        f_body = force @ self.drive + roller @ self.radial  # (n, 2), robot frame
        torque = R * force.sum(axis=1)

        speed = np.linalg.norm(vb[:, :2], axis=1, keepdims=True)
        direction = vb[:, :2] / np.maximum(speed, 1e-9)
        f_body -= p.body_viscous * vb[:, :2] + p.body_coulomb * direction * np.tanh(speed / 0.02)
        f_body[:, 1] -= p.lateral_viscous * vb[:, 1] + p.lateral_coulomb * np.tanh(vb[:, 1] / 0.02)
        torque -= p.spin_viscous * vb[:, 2] + 0.01 * np.tanh(vb[:, 2] / 0.05)

        acc_body = f_body / p.mass
        alpha = torque / p.inertia
        c, s = np.cos(self.pose[:, 2]), np.sin(self.pose[:, 2])
        acc_world = np.stack([c * acc_body[:, 0] - s * acc_body[:, 1], s * acc_body[:, 0] + c * acc_body[:, 1]], 1)

        # Semi-implicit Euler, as in ssl-sim.
        self.vel[:, :2] += acc_world * h
        self.vel[:, 2] += alpha * h
        self.pose += self.vel * h
