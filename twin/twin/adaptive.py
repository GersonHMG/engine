"""Context-adaptive twin (docs/digital-twin-generalization.md §2).

No robot ID: a context encoder reads a recording of the robot driving and
outputs a dynamics code z (mean and variance). z sets the parameters of a
wheel-level physics prior and conditions (FiLM) a GRU that corrects it.

    z_mu, z_logvar = model.encode(context, mask)
    params = model.physics(z)
    v_next, state = model.step(state, v, cmd_hist, z, params)

Inputs and outputs are as for models.GRUTwin: body velocity (vt, vn, omega)
in, body velocity after one tick out (in the robot frame of the current tick);
the pose is integrated outside.
"""

import math

import numpy as np
import torch
from torch import nn

from .models import HISTORY, rotate
from .population import PRIVILEGED, WHEEL_ANGLES, WHEEL_R
from .robots import TICK

Z_DIM = 8
N_DELAYS = HISTORY  # soft command delay over 0 … HISTORY - 1 ticks
FUTURE = HISTORY - 1  # ticks of response after each context tick (200 ms)
TRANSITION_DIM = 3 * HISTORY + 3 * (HISTORY + FUTURE)  # 200 ms of commands, ±200 ms of velocity

# Physics parameter layout: (name, size, low, high, nominal). Rates and limits
# are mapped log-uniformly into [low, high].
PHYSICS = (
    ("rate", 4, 2.0, 80.0, 20.0),  # per-wheel speed tracking, 1/s
    ("accel", 4, 0.3, 15.0, 5.0),  # per-wheel speed change limit, m/s^2
    ("top", 4, 0.5, 5.0, 3.5),  # per-wheel top speed, m/s
    ("lateral_damp", 1, 0.01, 15.0, 0.1),  # extra sideways damping, 1/s
    ("lateral_coulomb", 1, 0.001, 1.5, 0.01),  # extra sideways friction, m/s^2
    ("body_coulomb", 1, 0.001, 0.8, 0.1),  # friction on translation, m/s^2
    ("gyro", 1, 0.01, 6.0, 2.0),  # firmware spin-rate loop gain
)
N_PHYSICS = sum(size for _, size, *_ in PHYSICS) + N_DELAYS


def _logit_of(nominal, low, high):
    u = (math.log(nominal) - math.log(low)) / (math.log(high) - math.log(low))
    return math.log(u / (1 - u))


def soft_clamp(x, limit):
    """|x| ≤ limit, nearly linear below it, with gradients everywhere."""
    return x / (1 + (x / limit).abs() ** 6) ** (1 / 6)


class WheelPrior(nn.Module):
    """Kinematic model of the 4-wheel omni base, one step per tick.

    Each wheel's speed tracks what the (delayed) command asks with its own rate,
    acceleration limit and top speed; the body velocity is the least-squares
    fit of the wheel speeds. Extra sideways damping and friction cover jammed
    rollers. A weak wheel is a low rate / limit on that wheel, so faults are
    parameter changes, not something for the network to memorize."""

    def __init__(self):
        super().__init__()
        drive = np.stack([-np.sin(WHEEL_ANGLES), np.cos(WHEEL_ANGLES)], axis=1)
        J = np.concatenate([drive, np.full((4, 1), WHEEL_R)], axis=1)  # (4, 3): body velocity -> wheel speed
        self.register_buffer("J", torch.tensor(J, dtype=torch.float32))
        self.register_buffer("J_pinv", torch.tensor(np.linalg.pinv(J), dtype=torch.float32))

    @staticmethod
    def unpack(raw: torch.Tensor) -> dict:
        out, i = {}, 0
        for name, size, low, high, _ in PHYSICS:
            u = torch.sigmoid(raw[:, i : i + size])
            out[name] = torch.exp(math.log(low) + u * (math.log(high) - math.log(low)))
            i += size
        out["delay"] = torch.softmax(raw[:, i : i + N_DELAYS], dim=-1)  # (B, N_DELAYS) weights over 0 … 12 ticks
        return out

    def forward(self, v, cmd_hist, p):
        # Soft delay: a mixture of the last HISTORY commands (newest is index -1).
        newest_first = cmd_hist.flip(1)  # (B, HISTORY, 3)
        u = (p["delay"][..., None] * newest_first).sum(1)
        s = v @ self.J.T
        # Firmware gyro loop: the wheels also correct the spin-rate error.
        u = torch.cat([u[:, :2], u[:, 2:] + p["gyro"] * (u[:, 2:] - v[:, 2:])], dim=-1)
        target = u @ self.J.T
        gain = 1 - torch.exp(-p["rate"] * TICK)
        step_limit = p["accel"] * TICK
        step = step_limit * torch.tanh(gain * (target - s) / step_limit)
        s_new = soft_clamp(s + step, p["top"])
        v_new = s_new @ self.J_pinv.T
        vt, vn, w = v_new.unbind(-1)
        vn = vn * torch.exp(-p["lateral_damp"][:, 0] * TICK) - p["lateral_coulomb"][:, 0] * TICK * torch.tanh(vn / 0.02)
        speed = torch.sqrt(vt**2 + vn**2 + 1e-8)
        shrink = torch.clamp(1 - p["body_coulomb"][:, 0] * TICK / speed, min=0.0)
        return torch.stack([vt * shrink, vn * shrink, w], dim=-1)


class ContextEncoder(nn.Module):
    """q(z | context): per-transition MLP, attention pooling over time, and the
    amount of evidence. Order-free, any length, missing ticks masked."""

    def __init__(self, hidden=128, features=64):
        super().__init__()
        self.per_tick = nn.Sequential(
            nn.Linear(TRANSITION_DIM, hidden), nn.ReLU(), nn.Linear(hidden, hidden), nn.ReLU(), nn.Linear(hidden, features)
        )
        self.attend = nn.Linear(features, 1)
        self.out = nn.Sequential(nn.Linear(features + 1, hidden), nn.ReLU(), nn.Linear(hidden, 2 * Z_DIM))

    def forward(self, context, mask):
        """context (B, L, TRANSITION_DIM), mask (B, L) bool -> mu, logvar (B, Z_DIM).
        Rows with no context get the prior N(0, I)."""
        B = context.shape[0]
        if context.shape[1] == 0:
            zeros = torch.zeros(B, Z_DIM, device=context.device)
            return zeros, zeros
        h = self.per_tick(context)
        logits = self.attend(h)[..., 0].masked_fill(~mask, -1e9)
        weights = torch.softmax(logits, dim=1) * mask
        pooled = (weights[..., None] * h).sum(1)
        count = torch.log1p(mask.sum(1, keepdim=True).float()) / 8.0
        mu, logvar = self.out(torch.cat([pooled, count], -1)).chunk(2, -1)
        empty = mask.sum(1) == 0
        mu = torch.where(empty[:, None], torch.zeros_like(mu), mu)
        logvar = torch.where(empty[:, None], torch.zeros_like(logvar), logvar.clamp(-8, 4))
        return mu, logvar


class AdaptiveTwin(nn.Module):
    def __init__(self, hidden=96, layers=2):
        super().__init__()
        self.register_buffer("v_scale", torch.tensor([2.0, 2.0, 5.0]))
        self.encoder = ContextEncoder()
        self.prior = WheelPrior()
        self.physics_head = nn.Sequential(nn.Linear(Z_DIM, 64), nn.Tanh(), nn.Linear(64, N_PHYSICS))
        nominal = []
        for _, size, low, high, nom in PHYSICS:
            nominal += [_logit_of(nom, low, high)] * size
        delay_logits = [-((d - 6) / 2.0) ** 2 for d in range(N_DELAYS)]  # starts around 6 ticks
        nn.init.zeros_(self.physics_head[-1].weight)
        with torch.no_grad():
            self.physics_head[-1].weight.normal_(0, 0.01)
            self.physics_head[-1].bias.copy_(torch.tensor(nominal + delay_logits))
        self.hidden, self.layers = hidden, layers
        self.gru = nn.GRU(3 + 3 * HISTORY + 3 + Z_DIM, hidden, num_layers=layers, batch_first=True)
        self.film = nn.Linear(Z_DIM, 2 * hidden)
        nn.init.zeros_(self.film.weight)
        nn.init.zeros_(self.film.bias)
        self.head = nn.Sequential(nn.Linear(hidden + 6, 64), nn.Tanh(), nn.Dropout(0.05), nn.Linear(64, 3))
        nn.init.zeros_(self.head[-1].weight)
        nn.init.zeros_(self.head[-1].bias)
        self.privileged_head = nn.Sequential(nn.Linear(Z_DIM, 64), nn.Tanh(), nn.Linear(64, len(PRIVILEGED)))
        # Teacher (training in sim only): z from the true hardware parameters.
        self.teacher = nn.Sequential(nn.Linear(len(PRIVILEGED), 64), nn.ReLU(), nn.Linear(64, 64), nn.ReLU(), nn.Linear(64, Z_DIM))

    def encode(self, context, mask):
        return self.encoder(context, mask)

    def physics(self, z):
        return self.prior.unpack(self.physics_head(z))

    def init_state(self, batch, device):
        return torch.zeros(self.layers, batch, self.hidden, device=device)

    def _inputs(self, v, cmd_hist, prior_v, z):
        if v.dim() == 3:
            z = z[:, None].expand(-1, v.shape[1], -1)
        return torch.cat([v / self.v_scale, (cmd_hist / self.v_scale).flatten(-2), (prior_v - v) / (0.1 * self.v_scale), z], -1)

    def _correct(self, out, v, prior_v, z):
        gamma, beta = self.film(z).chunk(2, -1)
        out = out * (1 + gamma) + beta
        features = torch.cat([out, v / self.v_scale, (prior_v - v) / (0.1 * self.v_scale)], -1)
        return prior_v + self.head(features) * 0.1 * self.v_scale

    def warm_up(self, v_seq, hist_seq, z, params):
        B, W, _ = v_seq.shape
        flat = {k: t.repeat_interleave(W, 0) for k, t in params.items()}
        prior_v = self.prior(v_seq.reshape(-1, 3), hist_seq.reshape(-1, HISTORY, 3), flat).reshape(B, W, 3)
        _, h = self.gru(self._inputs(v_seq, hist_seq, prior_v, z), self.init_state(B, v_seq.device))
        return h

    def step(self, h, v, cmd_hist, z, params):
        prior_v = self.prior(v, cmd_hist, params)
        out, h = self.gru(self._inputs(v, cmd_hist, prior_v, z)[:, None], h)
        return self._correct(out[:, 0], v, prior_v, z), h


def history_windows(cmd, start, stop):
    return cmd[:, start - HISTORY + 1 : stop].unfold(1, HISTORY, 1).transpose(-1, -2)


def rollout(model, z, cmd, v_warm, pose0, warm, horizon, prefix):
    """As models.rollout, for the adaptive twin. cmd has `prefix` ticks of
    history before the warm-up."""
    params = model.physics(z)
    P = prefix
    h = model.warm_up(v_warm[:, :warm], history_windows(cmd, P, P + warm), z, params)
    hists = history_windows(cmd, P + warm, P + warm + horizon)
    v = v_warm[:, warm]
    xy, theta = pose0[:, :2], pose0[:, 2]
    poses, bodies = [], []
    for j in range(horizon):
        v_next, h = model.step(h, v, hists[:, j], z, params)
        xy = xy + rotate(v_next[:, :2], theta) * TICK
        theta = theta + v_next[:, 2] * TICK
        v = torch.cat([rotate(v_next[:, :2], -v_next[:, 2] * TICK), v_next[:, 2:]], dim=1)
        poses.append(torch.cat([xy, theta[:, None]], dim=1))
        bodies.append(v)
    return torch.stack(poses, 1), torch.stack(bodies, 1)


def transitions(cmd: np.ndarray, body: np.ndarray) -> np.ndarray:
    """Context features for every tick of a segment with a full command window
    before it and FUTURE ticks after it: (N, TRANSITION_DIM), float32."""
    ks = np.arange(HISTORY - 1, len(cmd) - FUTURE)
    return transition_features(cmd, body, ks)


def transition_features(cmd: np.ndarray, body: np.ndarray, ks: np.ndarray) -> np.ndarray:
    """For each tick k: commands k-12 … k and body velocities k-12 … k+12,
    the velocities expressed in the robot frame of tick k (heading changes
    integrated from omega). A local input/response clip of the robot."""
    scale = np.array([2.0, 2.0, 5.0])
    if len(ks) == 0:
        return np.zeros((0, TRANSITION_DIM), dtype=np.float32)
    c_off = np.arange(-(HISTORY - 1), 1)
    v_off = np.arange(-(HISTORY - 1), FUTURE + 1)
    windows = cmd[ks[:, None] + c_off] / scale  # (N, HISTORY, 3)
    v = body[ks[:, None] + v_off]  # (N, 25, 3)
    # Heading relative to tick k, from the integrated angular velocity.
    w = v[..., 2] * TICK
    rel = np.cumsum(w, axis=1)
    rel = rel - rel[:, HISTORY - 1 : HISTORY]
    c, s_ = np.cos(rel), np.sin(rel)
    vk = np.stack([c * v[..., 0] - s_ * v[..., 1], s_ * v[..., 0] + c * v[..., 1], v[..., 2]], -1) / scale
    return np.concatenate([windows.reshape(len(ks), -1), vk.reshape(len(ks), -1)], 1).astype(np.float32)
