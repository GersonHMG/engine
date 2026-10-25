"""Faulty robots as a linear map on the command: the robot executes M·u instead of u.

u = (vt, vn, ω) in the robot frame. The simulator's dynamics (acceleration and speed
limits) then act on M·u exactly as for a healthy robot.

Wheel fault: four omni wheels at angles ALPHA from the forward axis, at distance L from the
center, driving perpendicular to their radius. Wheel rim speeds are w = J·u with rows
J_i = (-sin α_i, cos α_i, L). A faulty wheel i only delivers k_i of its speed, and the body
motion is the least-squares fit of the four wheel speeds (the wheels slip equally):
    M = J⁺ · diag(k) · J            (k = 1: M = I)
Banana fault: strafing makes the robot turn, so a sideways command traces a curve:
    ω_actual = ω_cmd + κ·vn         (M = I with M[2, 1] = κ)
"""

import numpy as np

ALPHA = np.radians([60.0, 135.0, 225.0, 300.0])  # front-left, back-left, back-right, front-right
L = 0.08  # m, wheel distance from the center
WHEEL_NAMES = ["front-left", "back-left", "back-right", "front-right"]


def jacobian():
    return np.c_[-np.sin(ALPHA), np.cos(ALPHA), np.full(4, L)]


def wheel_matrix(efficiency, wheel=0):
    k = np.ones(4)
    k[wheel] = efficiency
    J = jacobian()
    return np.linalg.pinv(J) @ np.diag(k) @ J


def banana_matrix(kappa):
    M = np.eye(3)
    M[2, 1] = kappa
    return M


# The faults studied: name -> (kind, severity, matrix). "Severity" is 1 - efficiency for wheels.
FAULTS = {
    "wheel_0.75": ("wheel", 0.25, wheel_matrix(0.75)),
    "wheel_0.50": ("wheel", 0.50, wheel_matrix(0.50)),
    "wheel_0.00": ("wheel", 1.00, wheel_matrix(0.00)),
    "banana_0.25": ("banana", 0.25, banana_matrix(0.25)),
    "banana_0.50": ("banana", 0.50, banana_matrix(0.50)),
    "banana_1.00": ("banana", 1.00, banana_matrix(1.00)),
}
# One fault per kind is also used to train fault-specific models.
TRAINED = ["wheel_0.50", "banana_0.50"]


def apply(u, M):
    """Executed commands for commands u (..., 3)."""
    return u @ M.T
