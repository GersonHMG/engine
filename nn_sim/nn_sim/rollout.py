"""Open-loop rollouts in torch: feed the model its own predictions."""

import torch

# Scales of the rollout loss and of "acceptable" errors: 1 cm, 0.01 rad, 5 cm/s, 0.1 rad/s.
SCALES = dict(pos=0.01, theta=0.01, vel=0.05, omega=0.1)


def features(s, u):
    c, sn = torch.cos(s[..., 2]), torch.sin(s[..., 2])
    ub = c * s[..., 3] + sn * s[..., 4]
    vb = -sn * s[..., 3] + c * s[..., 4]
    return torch.stack([ub, vb, s[..., 5], u[..., 0], u[..., 1], u[..., 2]], -1)


def compose(s, d):
    c, sn = torch.cos(s[..., 2]), torch.sin(s[..., 2])
    return torch.stack(
        [
            s[..., 0] + c * d[..., 3] - sn * d[..., 4],
            s[..., 1] + sn * d[..., 3] + c * d[..., 4],
            s[..., 2] + d[..., 5],
            s[..., 3] + c * d[..., 0] - sn * d[..., 1],
            s[..., 4] + sn * d[..., 0] + c * d[..., 1],
            s[..., 5] + d[..., 2],
        ],
        -1,
    )


def rollout(model, s0, us):
    """States (B, N + 1, 6) from s0 (B, 6) and commands us (B, N, 3)."""
    out = [s0]
    s = s0
    for k in range(us.shape[1]):
        s = compose(s, model(features(s, us[:, k])))
        out.append(s)
    return torch.stack(out, 1)


def errors(pred, true):
    """Per-step errors (..., 4): position (m), heading (rad), velocity (m/s), ω (rad/s)."""
    pos = torch.linalg.norm(pred[..., :2] - true[..., :2], dim=-1)
    dth = pred[..., 2] - true[..., 2]
    theta = torch.abs(torch.atan2(torch.sin(dth), torch.cos(dth)))
    vel = torch.linalg.norm(pred[..., 3:5] - true[..., 3:5], dim=-1)
    omega = torch.abs(pred[..., 5] - true[..., 5])
    return torch.stack([pos, theta, vel, omega], -1)


def loss(pred, true):
    e = errors(pred[:, 1:], true[:, 1:])
    scale = torch.tensor([SCALES["pos"], SCALES["theta"], SCALES["vel"], SCALES["omega"]], device=e.device)
    return ((e / scale) ** 2).mean()
