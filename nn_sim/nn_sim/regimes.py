"""Dynamic regime of each transition, from the known limits (for analysis only).

Translational regime (mutually exclusive, in this order):
    stopped    no translation command and the robot at rest
    braking    no translation command, robot moving
    sat_accel  the clamped target velocity is farther than max_accel·DT: acceleration saturated all tick
    settling   the target is reached within the tick (the clamp switches off inside the tick)
    cruise     already at the target velocity
Tags (may overlap): cmd_above_limit (|command| > max_speed), at_max_speed,
rot_saturated (ω target farther than max_alpha·DT), turn_and_move (|ω| > 1 rad/s and |v| > 0.5 m/s).
"""

import numpy as np

from .data import DT, rotate
from .reference import DEFAULT, clamp_length

TRANSLATION = ["stopped", "braking", "sat_accel", "settling", "cruise"]
TAGS = ["cmd_above_limit", "at_max_speed", "rot_saturated", "turn_and_move"]


def label(s0, u, p=DEFAULT):
    th, vx, vy, w = s0[:, 2], s0[:, 3], s0[:, 4], s0[:, 5]
    cmd = np.hypot(u[:, 0], u[:, 1])
    speed = np.hypot(vx, vy)
    tx, ty = rotate(u[:, 0], u[:, 1], th)
    tx, ty = clamp_length(tx, ty, p["max_speed"])
    gap = np.hypot(tx - vx, ty - vy)

    reg = np.full(len(s0), 4)  # cruise
    reg[gap > 1e-9] = 3
    reg[gap > p["max_accel"] * DT + 1e-9] = 2
    reg[(cmd == 0) & (speed > 1e-3)] = 1
    reg[(cmd == 0) & (speed <= 1e-3)] = 0

    wgap = np.abs(np.clip(u[:, 2], -p["max_omega"], p["max_omega"]) - w)
    tags = {
        "cmd_above_limit": cmd > p["max_speed"],
        "at_max_speed": speed > p["max_speed"] - 1e-6,
        "rot_saturated": wgap > p["max_alpha"] * DT + 1e-9,
        "turn_and_move": (np.abs(w) > 1.0) & (speed > 0.5),
    }
    return reg, tags, gap
