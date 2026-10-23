"""Fit the baselines and train the GRU twin with a noise-aware rollout loss (plan §4).

    python -m twin.train --data data/prepared.pkl --out checkpoints
"""

import argparse
import math
import pickle
import time
from pathlib import Path

import numpy as np
import torch
from torch.nn import functional as F

from .data import Segment
from .models import ARX, MAX_DELAY, GRUTwin, Prior, PriorModel, rollout
from .robots import TICK

WARM = 30  # ticks of measured history before a free-run


class Windows:
    """Random or regular windows over prepared segments, as tensors."""

    def __init__(self, segments: list[Segment], robot_index: dict):
        self.segments = segments
        self.robot_index = robot_index
        self.lengths = np.array([len(s.cmd) for s in segments])

    def _starts(self, horizon):
        lo = WARM + MAX_DELAY
        return [(i, lo, len(s.cmd) - horizon - 1) for i, s in enumerate(self.segments) if len(s.cmd) - horizon - 1 > lo]

    def sample(self, batch: int, horizon: int, rng: np.random.Generator):
        ranges = self._starts(horizon)
        weights = np.array([b - a for _, a, b in ranges], dtype=float)
        pick = rng.choice(len(ranges), batch, p=weights / weights.sum())
        items = [(ranges[p][0], rng.integers(ranges[p][1], ranges[p][2])) for p in pick]
        return self.collate(items, horizon)

    def regular(self, horizon: int, every: int):
        items = [(i, s) for i, a, b in self._starts(horizon) for s in range(a, b, every)]
        return items

    def collate(self, items, horizon: int):
        def stack(get):
            return torch.tensor(np.stack([get(self.segments[i], s) for i, s in items]), dtype=torch.float32)

        P = MAX_DELAY
        batch = {
            "robot": torch.tensor([self.robot_index[self.segments[i].robot] for i, _ in items]),
            "cmd": stack(lambda g, s: g.cmd[s - WARM - P : s + horizon]),
            "v_warm": stack(lambda g, s: g.body[s - WARM : s + 1]),
            "pose0": stack(lambda g, s: g.pose[s]),
            "raw": stack(lambda g, s: g.raw[s + 1 : s + 1 + horizon]),
            "body": stack(lambda g, s: g.body[s + 1 : s + 1 + horizon]),
            "body_std": stack(lambda g, s: g.body_std[s + 1 : s + 1 + horizon]),
        }
        if self.segments[items[0][0]].truth_pose is not None:
            batch["truth_pose"] = stack(lambda g, s: g.truth_pose[s + 1 : s + 1 + horizon])
            batch["truth_body"] = stack(lambda g, s: g.truth_body[s + 1 : s + 1 + horizon])
            batch["truth_pose0"] = stack(lambda g, s: g.truth_pose[s])
        return batch


def run(model, batch, horizon, warm=WARM):
    return rollout(model, batch["robot"], batch["cmd"], batch["v_warm"], batch["pose0"], warm, horizon)


def noise_loss(poses, bodies, batch, noise_std, velocity_weight=0.2):
    """Huber loss on raw vision poses, in units of the vision noise, plus a
    one-step-style term on smoothed velocities weighted by the smoother's
    confidence. Missing detections are masked."""
    raw = batch["raw"]
    seen = ~torch.isnan(raw[..., 0])
    raw = torch.nan_to_num(raw)
    sig = noise_std[batch["robot"]][:, None, :]  # (B, 1, 3)
    err = (poses - raw) / sig
    pose_loss = (F.huber_loss(err, torch.zeros_like(err), reduction="none").sum(-1) * seen).sum() / seen.sum()
    floor = torch.tensor([0.02, 0.02, 0.05])
    verr = (bodies - batch["body"]) / (batch["body_std"] + floor)
    vel_loss = F.huber_loss(verr, torch.zeros_like(verr), reduction="none").sum(-1).mean()
    return pose_loss + velocity_weight * vel_loss


def estimate_delays(segments: list[Segment], robots: list[str], max_lag: int = MAX_DELAY) -> list[int]:
    """Lag (ticks) that best aligns command changes with velocity changes (plan §3.4)."""
    delays = []
    for robot in robots:
        scores = np.zeros(max_lag + 1)
        for seg in (s for s in segments if s.robot == robot):
            du = np.diff(seg.cmd, axis=0)
            dv = np.diff(seg.body, axis=0)
            for lag in range(max_lag + 1):
                a = du[: len(du) - lag]
                b = dv[lag:]
                scores[lag] += sum(np.dot(a[:, i], b[:, i]) / (np.linalg.norm(a[:, i]) * np.linalg.norm(b[:, i]) + 1e-9) for i in range(3))
        delays.append(int(np.argmax(scores)))
    return delays


@torch.no_grad()
def per_robot_loss(model, windows, noise_std, robots, horizon=30):
    batch = windows.collate(windows.regular(horizon, 45), horizon)
    poses, bodies = run(model, batch, horizon)
    losses = []
    for r in range(len(robots)):
        mask = batch["robot"] == r
        sub = {k: v[mask] for k, v in batch.items()}
        losses.append(noise_loss(poses[mask], bodies[mask], sub, noise_std).item())
    return np.array(losses)


def fit_delays(train, val, noise_std, robots, rng, candidates=range(2, MAX_DELAY - 1)):
    """Per robot, the command delay whose quickly fitted prior predicts validation best."""
    scores = []
    for d in candidates:
        model = PriorModel(Prior(len(robots), [d] * len(robots)))
        opt = torch.optim.Adam(model.parameters(), lr=3e-2)
        for _ in range(150):
            b = train.sample(128, 30, rng)
            poses, bodies = run(model, b, 30)
            loss = noise_loss(poses, bodies, b, noise_std)
            opt.zero_grad()
            loss.backward()
            opt.step()
        scores.append(per_robot_loss(model, val, noise_std, robots))
        print(f"  delay {d}: val loss per robot {np.round(scores[-1], 2)}", flush=True)
    scores = np.array(scores)
    return [int(list(candidates)[i]) for i in scores.argmin(axis=0)]


def fit_arx(segments: list[Segment], robots: list[str], history: int = 12) -> ARX:
    """Least squares per robot on smoothed one-step transitions."""
    model = ARX(len(robots), history)
    for r, robot in enumerate(robots):
        X, Y = [], []
        for seg in (s for s in segments if s.robot == robot):
            L = len(seg.cmd)
            for k in range(history, L - 1):
                hist = seg.cmd[k - history + 1 : k + 1][::-1]
                X.append(np.concatenate([seg.body[k], hist.ravel(), [1.0]]))
                nxt = seg.body[k + 1]
                a = nxt[2] * TICK  # back into the frame of tick k
                Y.append([np.cos(a) * nxt[0] - np.sin(a) * nxt[1], np.sin(a) * nxt[0] + np.cos(a) * nxt[1], nxt[2]])
        X, Y = np.array(X), np.array(Y)
        reg = 1e-3 * np.eye(X.shape[1])
        W = np.linalg.solve(X.T @ X + reg, X.T @ Y)
        model.weights.data[r] = torch.tensor(W, dtype=torch.float32)
    return model


def perturb(batch, noise_std, scale=1.0):
    """Input noise injection: the start state the model is given is never exact,
    neither from the smoother nor from its own earlier predictions."""
    b = dict(batch)
    sig = noise_std[b["robot"]]
    b["pose0"] = b["pose0"] + torch.randn_like(b["pose0"]) * sig * scale
    vel_noise = torch.tensor([0.015, 0.015, 0.1]) * scale
    b["v_warm"] = b["v_warm"] + torch.randn_like(b["v_warm"]) * vel_noise
    return b


def train_rollout(model, params, train, val, noise_std, schedule, lr, rng, batch=128, log="", val_horizon=60, noise_injection=False):
    """Adam on the free-run rollout loss with a horizon curriculum: the model is
    fed its own predictions, as when it runs as a simulator. Keeps the best
    validation state."""
    opt = torch.optim.AdamW(params, lr=lr, weight_decay=1e-4)
    total = sum(n for _, n in schedule)
    sched = torch.optim.lr_scheduler.LambdaLR(opt, lambda i: 0.5 * (1 + math.cos(math.pi * min(i, total) / total)))
    items = val.regular(val_horizon, 45)
    items = [items[i] for i in np.linspace(0, len(items) - 1, min(len(items), 600)).astype(int)]
    val_batch = val.collate(items, val_horizon)
    best, best_state, step = float("inf"), None, 0
    start = time.time()
    for horizon, iters in schedule:
        for _ in range(iters):
            model.train()
            b = train.sample(batch, horizon, rng)
            if noise_injection:
                b = perturb(b, noise_std)
            poses, bodies = run(model, b, horizon)
            loss = noise_loss(poses, bodies, b, noise_std)
            opt.zero_grad()
            loss.backward()
            torch.nn.utils.clip_grad_norm_(params, 1.0)
            opt.step()
            sched.step()
            step += 1
            if step % 250 == 0 or step == total:
                model.eval()
                with torch.no_grad():
                    vp, vb = run(model, val_batch, val_horizon)
                    vloss = noise_loss(vp, vb, val_batch, noise_std).item()
                if vloss < best:
                    best, best_state = vloss, {k: v.clone() for k, v in model.state_dict().items()}
                print(f"{log} step {step:5d} h={horizon:3d} train {loss.item():8.3f} val({val_horizon * TICK:.0f} s) {vloss:8.3f} [{time.time() - start:5.0f} s]", flush=True)
    model.load_state_dict(best_state)
    return best


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--data", type=Path, default=Path("data/prepared.pkl"))
    parser.add_argument("--out", type=Path, default=Path("checkpoints"))
    parser.add_argument("--iters", type=int, default=9000, help="GRU training steps")
    parser.add_argument("--seed", type=int, default=0)
    args = parser.parse_args()

    torch.manual_seed(args.seed)
    rng = np.random.default_rng(args.seed)
    with open(args.data, "rb") as f:
        data = pickle.load(f)
    robots = sorted(data["noise"])
    index = {r: i for i, r in enumerate(robots)}
    noise_std = torch.tensor(np.stack([data["noise"][r] for r in robots]), dtype=torch.float32)
    train = Windows(data["splits"]["train"], index)
    val = Windows(data["splits"]["val"], index)
    args.out.mkdir(parents=True, exist_ok=True)
    meta = {"robots": robots, "noise_std": noise_std, "jerk": data["jerk"]}

    # Cross-correlation gives a first guess; the delay that fits best wins.
    guess = estimate_delays(data["splits"]["train"], robots)
    print("delay guess from cross-correlation (ticks):", dict(zip(robots, guess)))
    delays = fit_delays(train, val, noise_std, robots, rng)
    print("fitted delays (ticks):", dict(zip(robots, delays)))

    # Baseline 1: ssl-sim as configured (no delay).
    baseline1 = PriorModel(Prior(len(robots), [0] * len(robots), trainable=False))
    torch.save({"kind": "prior", "state": baseline1.state_dict(), **meta}, args.out / "baseline1_sslsim.pt")

    # Baseline 2: ssl-sim's model with fitted parameters and delay.
    baseline2 = PriorModel(Prior(len(robots), delays))
    train_rollout(baseline2, list(baseline2.parameters()), train, val, noise_std, [(10, 200), (30, 400)], 3e-2, rng, log="prior")
    for r, robot in enumerate(robots):
        p = baseline2.prior.params(torch.tensor([r]))[0].tolist()
        print(f"  {robot:10s} max_accel {p[0]:.2f} max_alpha {p[1]:.1f} max_speed {p[2]:.2f} max_omega {p[3]:.2f} delay {delays[r]}")
    torch.save({"kind": "prior", "state": baseline2.state_dict(), **meta}, args.out / "baseline2_fitted.pt")

    # Baseline 3: linear ARX.
    arx = fit_arx(data["splits"]["train"], robots)
    torch.save({"kind": "arx", "state": arx.state_dict(), **meta}, args.out / "baseline3_arx.pt")

    # GRU twin on top of the fitted prior (kept fixed).
    prior = Prior(len(robots), delays, trainable=False)
    prior.load_state_dict(baseline2.prior.state_dict())
    twin = GRUTwin(prior, len(robots))
    params = [p for p in twin.parameters() if p.requires_grad]
    print("GRU twin parameters:", sum(p.numel() for p in params))
    n = args.iters
    # Curriculum up to 2 s of free-running on its own predictions.
    schedule = [(10, n // 6), (30, n // 6), (60, n // 3), (120, n // 3)]
    train_rollout(twin, params, train, val, noise_std, schedule, 2e-3, rng, log="twin ", val_horizon=120, noise_injection=True)
    torch.save({"kind": "gru", "state": twin.state_dict(), "delays": delays, "hidden": twin.hidden, **meta}, args.out / "twin_gru.pt")


if __name__ == "__main__":
    main()
