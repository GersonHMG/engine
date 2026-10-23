"""Dynamics models: physics prior, linear ARX and the GRU residual twin (plan §4).

All models share one step interface so they are trained and evaluated by the
same rollout code:

    state = model.init_state(batch)
    v_next, state = model.step(state, v, cmd_hist, cmd_delayed, robot)

``cmd_hist`` holds the last HISTORY commands, oldest first, the newest being
tick k's. ``v`` is the body velocity (vt, vn, omega) at tick k; ``v_next`` is the body
velocity after the tick, expressed in the robot frame of tick k (the rollout
rotates it into the new frame and integrates the pose).
"""

import torch
from torch import nn

from .robots import TICK

V_SCALE = torch.tensor([2.0, 2.0, 5.0])  # feature normalization, m/s, m/s, rad/s
MAX_DELAY = 12  # ticks of command history kept before every window
HISTORY = MAX_DELAY + 1  # commands the models see: 200 ms

# ssl-sim defaults (simulator.toml): max_accel, max_alpha, max_speed, max_omega.
SSL_SIM_DEFAULTS = (6.0, 40.0, 3.5, 10.0)


def clamp_length(v: torch.Tensor, max_len: torch.Tensor) -> torch.Tensor:
    length = v.norm(dim=-1, keepdim=True).clamp_min(1e-9)
    return v * torch.clamp(max_len / length, max=1.0)


class Prior(nn.Module):
    """ssl-sim's robot model (Robot::drive) plus a pure command delay.

    Per robot: max_accel, max_alpha, max_speed, max_omega (fitted or fixed).
    """

    def __init__(self, n_robots: int, delays, init=SSL_SIM_DEFAULTS, trainable=True):
        super().__init__()
        log_params = torch.log(torch.tensor(init, dtype=torch.float32)).repeat(n_robots, 1)
        self.log_params = nn.Parameter(log_params, requires_grad=trainable)
        self.register_buffer("delays", torch.as_tensor(delays, dtype=torch.long))

    def params(self, robot):
        return torch.exp(self.log_params[robot])

    def forward(self, v, cmd_delayed, robot):
        accel, alpha, speed, omega_max = self.params(robot).unbind(-1)
        target = clamp_length(cmd_delayed[:, :2], speed[:, None])
        vt_vn = v[:, :2] + clamp_length(target - v[:, :2], (accel * TICK)[:, None])
        target_w = torch.maximum(torch.minimum(cmd_delayed[:, 2], omega_max), -omega_max)
        dw = target_w - v[:, 2]
        w = v[:, 2] + torch.maximum(torch.minimum(dw, alpha * TICK), -alpha * TICK)
        return torch.cat([vt_vn, w[:, None]], dim=1)


class PriorModel(nn.Module):
    """Baselines 1 and 2: the prior alone."""

    def __init__(self, prior: Prior):
        super().__init__()
        self.prior = prior

    @property
    def delays(self):
        return self.prior.delays

    def init_state(self, batch: int):
        return None

    def warm_up(self, v_seq, hist_seq, cmd_delayed_seq, robot):
        return None

    def step(self, state, v, cmd_hist, cmd_delayed, robot):
        return self.prior(v, cmd_delayed, robot), state


class ARX(nn.Module):
    """Baseline 3: linear map from (v_k, last H commands) to v_{k+1}, per robot."""

    def __init__(self, n_robots: int, history: int = 12):
        super().__init__()
        self.history = history
        self.weights = nn.Parameter(torch.zeros(n_robots, 3 + 3 * history + 1, 3))
        self.register_buffer("delays", torch.zeros(n_robots, dtype=torch.long))

    def init_state(self, batch: int):
        return None

    def warm_up(self, v_seq, hist_seq, cmd_delayed_seq, robot):
        return None

    def step(self, state, v, cmd_hist, cmd_delayed, robot):
        newest_first = cmd_hist[:, -self.history :].flip(1)
        x = torch.cat([v, newest_first.reshape(v.shape[0], -1), torch.ones(v.shape[0], 1)], dim=1)
        return torch.einsum("bi,bio->bo", x, self.weights[robot]), state


class GRUTwin(nn.Module):
    """Plan §4.2: fitted prior + GRU residual, with a per-robot embedding."""

    def __init__(self, prior: Prior, n_robots: int, hidden: int = 96, layers: int = 2, embedding: int = 8):
        super().__init__()
        self.prior = prior
        self.embedding = nn.Embedding(n_robots, embedding)
        self.hidden = hidden
        self.layers = layers
        self.gru = nn.GRU(3 * (HISTORY + 3) + embedding, hidden, num_layers=layers, batch_first=True)
        self.head = nn.Sequential(nn.Linear(hidden + 9, 64), nn.Tanh(), nn.Dropout(0.05), nn.Linear(64, 3))
        nn.init.zeros_(self.head[-1].weight)
        nn.init.zeros_(self.head[-1].bias)

    @property
    def delays(self):
        return self.prior.delays

    def init_state(self, batch: int):
        return torch.zeros(self.layers, batch, self.hidden)

    def _inputs(self, v, cmd_hist, cmd_delayed, prior_v, robot):
        e = self.embedding(robot)
        if v.dim() == 3:
            e = e[:, None].expand(-1, v.shape[1], -1)
        hist = (cmd_hist / V_SCALE).flatten(-2)
        return torch.cat([v / V_SCALE, hist, cmd_delayed / V_SCALE, (prior_v - v) / (V_SCALE * 0.1), e], -1)

    def warm_up(self, v_seq, hist_seq, cmd_delayed_seq, robot):
        """Runs the GRU over measured history (teacher forcing). v_seq (B, W, 3), hist_seq (B, W, HISTORY, 3)."""
        B, W, _ = v_seq.shape
        prior_v = self.prior(v_seq.reshape(-1, 3), cmd_delayed_seq.reshape(-1, 3), robot.repeat_interleave(W)).reshape(B, W, 3)
        _, h = self.gru(self._inputs(v_seq, hist_seq, cmd_delayed_seq, prior_v, robot), self.init_state(B))
        return h

    def step(self, h, v, cmd_hist, cmd_delayed, robot):
        prior_v = self.prior(v, cmd_delayed, robot)
        x = self._inputs(v, cmd_hist, cmd_delayed, prior_v, robot)
        out, h = self.gru(x[:, None], h)
        features = torch.cat([out[:, 0], v / V_SCALE, cmd_delayed / V_SCALE, (prior_v - v) / (V_SCALE * 0.1)], dim=1)
        return prior_v + self.head(features) * V_SCALE * 0.1, h


def rotate(xy: torch.Tensor, angle: torch.Tensor) -> torch.Tensor:
    c, s = torch.cos(angle), torch.sin(angle)
    return torch.stack([c * xy[..., 0] - s * xy[..., 1], s * xy[..., 0] + c * xy[..., 1]], dim=-1)


def delayed_commands(cmd: torch.Tensor, delays: torch.Tensor, robot: torch.Tensor, start: int, stop: int):
    """cmd[:, k - delay] for k in [start, stop), per batch row's robot delay."""
    d = delays[robot]  # (B,)
    idx = torch.arange(start, stop)[None, :] - d[:, None]  # (B, W)
    return torch.gather(cmd, 1, idx.clamp_min(0)[..., None].expand(-1, -1, 3))


def history_windows(cmd: torch.Tensor, start: int, stop: int) -> torch.Tensor:
    """(B, stop - start, HISTORY, 3): the HISTORY commands up to each tick in [start, stop)."""
    return cmd[:, start - HISTORY + 1 : stop].unfold(1, HISTORY, 1).transpose(-1, -2)


def rollout(model, robot, cmd, v_warm, pose0, warm: int, horizon: int):
    """Free-runs `model` for `horizon` ticks after `warm` ticks of measured history.

    cmd: (B, MAX_DELAY + warm + horizon, 3) commands, window starts after the
    MAX_DELAY history prefix. v_warm: (B, warm + 1, 3) measured body velocity for
    the warm-up ticks plus the start tick. pose0: (B, 3) pose at the start tick.
    Returns poses (B, horizon, 3) and body velocities (B, horizon, 3) for ticks
    start+1 … start+horizon.
    """
    P = MAX_DELAY
    delays = model.delays
    state = model.init_state(cmd.shape[0])
    if warm > 0:
        state = model.warm_up(
            v_warm[:, :warm], history_windows(cmd, P, P + warm), delayed_commands(cmd, delays, robot, P, P + warm), robot
        )
    later = delayed_commands(cmd, delays, robot, P + warm, P + warm + horizon)
    hists = history_windows(cmd, P + warm, P + warm + horizon)
    v = v_warm[:, warm]
    xy, theta = pose0[:, :2], pose0[:, 2]
    poses, bodies = [], []
    for j in range(horizon):
        v_next, state = model.step(state, v, hists[:, j], later[:, j], robot)
        # Semi-implicit Euler, as ssl-sim: new velocity moves the pose.
        xy = xy + rotate(v_next[:, :2], theta) * TICK
        theta = theta + v_next[:, 2] * TICK
        v = torch.cat([rotate(v_next[:, :2], -v_next[:, 2] * TICK), v_next[:, 2:]], dim=1)
        poses.append(torch.cat([xy, theta[:, None]], dim=1))
        bodies.append(v)
    return torch.stack(poses, 1), torch.stack(bodies, 1)
