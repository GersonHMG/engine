"""Models of Δ = f(z) in standardized space, plus the constant-velocity baseline.

Every model maps raw features z (..., 6) to raw targets Δ (..., 6); the
standardization lives inside the model so rollouts can call it directly.
"""

import torch
from torch import nn

from .data import DT


class Standardized(nn.Module):
    def __init__(self, z_mean, z_std, d_mean, d_std):
        super().__init__()
        for name, value in dict(z_mean=z_mean, z_std=z_std, d_mean=d_mean, d_std=d_std).items():
            self.register_buffer(name, torch.as_tensor(value, dtype=torch.float32))

    def forward(self, z):
        zn = (z.float() - self.z_mean) / self.z_std
        return (self.core(zn) * self.d_std + self.d_mean).to(z.dtype)

    def forward_normalized(self, zn):
        return self.core(zn)


class MLP(Standardized):
    def __init__(self, stats, hidden=256, layers=3):
        super().__init__(*stats)
        dims = [6] + [hidden] * layers
        blocks = []
        for a, b in zip(dims[:-1], dims[1:]):
            blocks += [nn.Linear(a, b), nn.SiLU()]
        blocks.append(nn.Linear(dims[-1], 6))
        self.core = nn.Sequential(*blocks)


class Linear(Standardized):
    """Least squares on standardized z (closed form, small ridge)."""

    def __init__(self, stats):
        super().__init__(*stats)
        self.core = nn.Linear(6, 6)

    @torch.no_grad()
    def fit(self, z, d, ridge=1e-6):
        zn = (z.float() - self.z_mean) / self.z_std
        dn = (d.float() - self.d_mean) / self.d_std
        x = torch.cat([zn, torch.ones_like(zn[:, :1])], 1).double()
        w = torch.linalg.solve(x.T @ x + ridge * torch.eye(7, dtype=x.dtype, device=x.device), x.T @ dn.double())
        self.core.weight.copy_(w[:6].T.float())
        self.core.bias.copy_(w[6].float())
        return self


class ConstantVelocity(nn.Module):
    """The robot keeps its velocity: no velocity change, pose moves by v·DT."""

    def forward(self, z):
        zero = torch.zeros_like(z[..., 0])
        return torch.stack([zero, zero, zero, z[..., 0] * DT, z[..., 1] * DT, z[..., 2] * DT], -1)


def n_params(model):
    return sum(p.numel() for p in model.parameters())
