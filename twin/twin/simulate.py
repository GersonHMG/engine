"""Use a trained model as a simulator of one robot (plan §5).

    twin = Twin.load("checkpoints/twin_gru.pt", robot="bad_wheel")
    twin.reset(0.0, 0.0, 0.0)
    for cmd in commands:              # (vt, vn, omega), robot frame
        state = twin.step(cmd)        # x, y, theta, vt, vn, omega after 1/60 s
"""

import numpy as np
import torch

from .models import ARX, HISTORY, GRUTwin, Prior, PriorModel, rotate
from .robots import TICK, quantize
from .train import WARM


def load_model(path):
    ckpt = torch.load(path, weights_only=False)
    n = len(ckpt["robots"])
    if ckpt["kind"] == "prior":
        model = PriorModel(Prior(n, [0] * n, trainable=False))
    elif ckpt["kind"] == "arx":
        model = ARX(n)
    elif ckpt["kind"] == "gru":
        model = GRUTwin(Prior(n, ckpt["delays"], trainable=False), n, hidden=ckpt.get("hidden", 96))
    else:
        raise ValueError(ckpt["kind"])
    model.load_state_dict(ckpt["state"])
    model.eval()
    return model, ckpt


class Twin:
    """One simulated robot driven by a trained dynamics model. Deterministic."""

    def __init__(self, model, robots, robot: str):
        self.model = model
        self.robot = torch.tensor([robots.index(robot)])
        self.reset(0.0, 0.0, 0.0)

    @classmethod
    def load(cls, path, robot: str):
        model, ckpt = load_model(path)
        return cls(model, ckpt["robots"], robot)

    @torch.no_grad()
    def reset(self, x: float, y: float, theta: float):
        """Robot at rest at this pose, with an at-rest command history."""
        self.history = torch.zeros(1, HISTORY, 3)
        zeros = torch.zeros(1, WARM, 3)
        self.state = self.model.warm_up(zeros, torch.zeros(1, WARM, HISTORY, 3), zeros, self.robot)
        self.xy = torch.tensor([[x, y]], dtype=torch.float32)
        self.theta = torch.tensor([theta], dtype=torch.float32)
        self.v = torch.zeros(1, 3)

    @torch.no_grad()
    def step(self, command) -> np.ndarray:
        cmd = torch.tensor(quantize(np.asarray(command, dtype=np.float64)), dtype=torch.float32).reshape(1, 3)
        self.history = torch.cat([self.history[:, 1:], cmd[:, None]], dim=1)
        delayed = self.history[:, -1 - self.model.delays[self.robot][0]]
        v_next, self.state = self.model.step(self.state, self.v, self.history, delayed, self.robot)
        self.xy = self.xy + rotate(v_next[:, :2], self.theta) * TICK
        self.theta = self.theta + v_next[:, 2] * TICK
        self.v = torch.cat([rotate(v_next[:, :2], -v_next[:, 2] * TICK), v_next[:, 2:]], dim=1)
        return self.observe()

    def observe(self) -> np.ndarray:
        return np.concatenate([self.xy[0].numpy(), self.theta.numpy(), self.v[0].numpy()]).astype(np.float64)

    def rollout(self, pose, commands) -> np.ndarray:
        """(T, 6) states after each of the (T, 3) commands, from rest at `pose`."""
        self.reset(*pose)
        return np.array([self.step(c) for c in commands])
