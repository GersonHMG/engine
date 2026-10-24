"""Train the context-adaptive twin on the robot population
(docs/digital-twin-generalization.md §3.1–3.2).

    python -m twin.train_adaptive prepare            # clean + smooth every session (once)
    python -m twin.train_adaptive train --device cuda

Each sample pairs a context (a stretch of one session of a robot) with a
target window from the robot's *other* session: z can only help by holding
what is constant about the robot.
"""

import argparse
import json
import math
import pickle
import time
from concurrent.futures import ProcessPoolExecutor
from pathlib import Path

import numpy as np
import torch
from torch.nn import functional as F

from .adaptive import HISTORY, TRANSITION_DIM, AdaptiveTwin, rollout
from .data import load_session, prepare, wrap

WARM = 30
PREFIX = HISTORY - 1
JERK = np.array([30.0, 30.0, 3000.0])  # smoother tuning found by likelihood on the first data set
STILL_TICKS = 600  # sessions start with 10 s at rest
SCALE = np.array([2.0, 2.0, 5.0])

RECOVERY_FRACTION = 0.25
RECOVERY_KICK = np.array([0.3, 0.3, 1.5])
RECOVERY_GRACE = 15


# --- preparation ----------------------------------------------------------


def session_noise(raw: np.ndarray) -> np.ndarray:
    """Vision noise std from the at-rest start of the session (robust)."""
    still = raw[:STILL_TICKS]
    still = still[~np.isnan(still[:, 0])]
    centre = np.array([np.median(still[:, 0]), np.median(still[:, 1]), np.angle(np.mean(np.exp(1j * still[:, 2])))])
    dev = still - centre
    dev[:, 2] = wrap(dev[:, 2])
    return np.maximum(1.4826 * np.median(np.abs(dev), axis=0), [5e-4, 5e-4, 2e-3])


def prepare_one(path: Path) -> dict:
    session = load_session(path)
    meta = json.loads(path.with_suffix(".json").read_text())
    noise = session_noise(session.raw)
    segs = prepare(session, noise, JERK)
    f32 = lambda a: np.asarray(a, dtype=np.float32)
    return {
        "robot": meta["robot"],
        "session": meta["program"],
        "privileged": f32(meta["privileged"]),
        "noise": f32(noise),
        "segments": [
            {k: f32(getattr(g, k)) for k in ("cmd", "raw", "pose", "body", "body_std", "truth_pose", "truth_body")}
            for g in segs
        ],
    }


def cmd_prepare(args):
    out = {}
    for split in ("train", "val", "test"):
        paths = sorted((args.data / split).glob("*/*.csv"))
        start = time.time()
        with ProcessPoolExecutor() as pool:
            sessions = list(pool.map(prepare_one, paths, chunksize=4))
        out[split] = sessions
        ticks = sum(len(g["cmd"]) for s in sessions for g in s["segments"])
        print(f"{split}: {len(sessions)} sessions, {ticks / 3600:.0f} min clean, {time.time() - start:.0f} s", flush=True)
    with open(args.prepared, "wb") as f:
        pickle.dump(out, f)


# --- sampling -------------------------------------------------------------


class Session:
    """One prepared session, with its segments concatenated for fast context slicing."""

    def __init__(self, d: dict):
        self.robot, self.name = d["robot"], d["session"]
        self.privileged, self.noise = d["privileged"], d["noise"]
        self.segments = d["segments"]
        cmd, body, idx, offset = [], [], [], 0
        for g in self.segments:
            L = len(g["cmd"])
            cmd.append(g["cmd"])
            body.append(g["body"])
            idx.append(np.arange(offset + HISTORY - 1, offset + L - 1))
            offset += L
        self.cmd = np.concatenate(cmd) if cmd else np.zeros((0, 3), np.float32)
        self.body = np.concatenate(body) if body else np.zeros((0, 3), np.float32)
        self.idx = np.concatenate(idx) if idx else np.zeros(0, int)  # ticks usable as transitions

    def context(self, length: int, rng, start=None) -> np.ndarray:
        """`length` consecutive usable transitions (fewer if the session is short)."""
        n = len(self.idx)
        length = min(length, n)
        if length == 0:
            return np.zeros((0, TRANSITION_DIM), np.float32)
        if start is None:
            start = int(rng.integers(0, n - length + 1))
        ks = self.idx[start : start + length]
        return transition_features(self.cmd, self.body, ks)

    def windows(self, horizon: int):
        lo = WARM + PREFIX
        return [(i, s) for i, g in enumerate(self.segments) for s in range(lo, len(g["cmd"]) - horizon - 1)]


def transition_features(cmd, body, ks):
    offs = np.arange(-(HISTORY - 1), 1)
    windows = cmd[ks[:, None] + offs] / SCALE  # (N, HISTORY, 3)
    v = body[ks]
    nxt = body[ks + 1]
    a = nxt[:, 2] * (1 / 60)
    nxt_k = np.stack([np.cos(a) * nxt[:, 0] - np.sin(a) * nxt[:, 1], np.sin(a) * nxt[:, 0] + np.cos(a) * nxt[:, 1], nxt[:, 2]], 1)
    return np.concatenate([windows.reshape(len(ks), -1), v / SCALE, (nxt_k - v) / (0.1 * SCALE)], 1).astype(np.float32)


def target_window(sess: Session, seg: int, s: int, horizon: int) -> dict:
    g = sess.segments[seg]
    out = {
        "cmd": g["cmd"][s - WARM - PREFIX : s + horizon],
        "v_warm": g["body"][s - WARM : s + 1],
        "pose0": g["pose"][s],
        "raw": g["raw"][s + 1 : s + 1 + horizon],
        "body": g["body"][s + 1 : s + 1 + horizon],
        "body_std": g["body_std"][s + 1 : s + 1 + horizon],
        "truth_pose": g["truth_pose"][s + 1 : s + 1 + horizon],
        "truth_body": g["truth_body"][s + 1 : s + 1 + horizon],
        "noise": sess.noise,
        "privileged": sess.privileged,
    }
    return out


def collate(targets: list, contexts: list, device) -> dict:
    batch = {k: torch.from_numpy(np.stack([t[k] for t in targets])).to(device) for k in targets[0]}
    L = max(1, max(len(c) for c in contexts))
    ctx = np.zeros((len(contexts), L, TRANSITION_DIM), np.float32)
    mask = np.zeros((len(contexts), L), bool)
    for i, c in enumerate(contexts):
        ctx[i, : len(c)] = c
        mask[i, : len(c)] = True
    if max(len(c) for c in contexts) == 0:
        ctx, mask = ctx[:, :0], mask[:, :0]
    batch["context"] = torch.from_numpy(ctx).to(device)
    batch["mask"] = torch.from_numpy(mask).to(device)
    return batch


class Population:
    def __init__(self, sessions: list[dict]):
        self.by_robot = {}
        for d in sessions:
            self.by_robot.setdefault(d["robot"], []).append(Session(d))
        self.robots = sorted(self.by_robot)
        self._windows = {}

    def windows(self, sess: Session, horizon: int):
        key = (id(sess), horizon)
        if key not in self._windows:
            self._windows[key] = sess.windows(horizon)
        return self._windows[key]

    def sample(self, batch: int, horizon: int, context_len: int, rng, device):
        targets, contexts = [], []
        while len(targets) < batch:
            sessions = self.by_robot[self.robots[rng.integers(len(self.robots))]]
            order = rng.permutation(len(sessions))
            tgt, ctx = sessions[order[0]], sessions[order[-1]]
            wins = self.windows(tgt, horizon)
            if not wins:
                continue
            seg, s = wins[rng.integers(len(wins))]
            targets.append(target_window(tgt, seg, s, horizon))
            contexts.append(ctx.context(context_len, rng))
        return collate(targets, contexts, device)


# --- losses ---------------------------------------------------------------


def perturb(batch, rng_t, recovery=True):
    b = dict(batch)
    sig = b["noise"]
    b["pose0"] = b["pose0"] + torch.randn_like(b["pose0"]) * sig
    vel_noise = torch.tensor([0.015, 0.015, 0.1], device=sig.device)
    b["v_warm"] = b["v_warm"] + torch.randn_like(b["v_warm"]) * vel_noise
    if recovery:
        kicked = torch.rand(b["v_warm"].shape[0], device=sig.device) < RECOVERY_FRACTION
        v = b["v_warm"].clone()
        v[kicked, -1] += torch.randn(int(kicked.sum()), 3, device=sig.device) * torch.tensor(RECOVERY_KICK, dtype=torch.float32, device=sig.device)
        b["v_warm"], b["kicked"] = v, kicked
    return b


def rollout_loss(poses, bodies, batch, velocity_weight=0.2):
    """As train.noise_loss, with each sample's own vision noise."""
    raw = batch["raw"]
    seen = ~torch.isnan(raw[..., 0])
    raw = torch.nan_to_num(raw)
    vel_mask = torch.ones_like(seen, dtype=torch.float32)
    kicked = batch.get("kicked")
    if kicked is not None:
        seen = seen & ~kicked[:, None]
        vel_mask[kicked, :RECOVERY_GRACE] = 0.0
    err = (poses - raw) / batch["noise"][:, None, :]
    pose_loss = (F.huber_loss(err, torch.zeros_like(err), reduction="none").sum(-1) * seen).sum() / seen.sum().clamp_min(1)
    floor = torch.tensor([0.02, 0.02, 0.05], device=raw.device)
    verr = (bodies - batch["body"]) / (batch["body_std"] + floor)
    per = F.huber_loss(verr, torch.zeros_like(verr), reduction="none").sum(-1)
    vel_loss = (per * vel_mask).sum() / vel_mask.sum()
    if kicked is not None:
        k = kicked[:, None].float() * vel_mask
        vel_loss = vel_loss + (per * k).sum() / k.sum().clamp_min(1)
    return pose_loss + velocity_weight * vel_loss


def run(model, batch, horizon, sample_z=False, z=None):
    mu, logvar = model.encode(batch["context"], batch["mask"])
    if z is None:
        z = mu + torch.randn_like(mu) * torch.exp(0.5 * logvar) if sample_z else mu
    poses, bodies = rollout(model, z, batch["cmd"], batch["v_warm"], batch["pose0"], WARM, horizon, PREFIX)
    return poses, bodies, mu, logvar, z


def total_loss(model, batch, horizon, beta=1e-3, privileged_weight=0.1):
    poses, bodies, mu, logvar, z = run(model, batch, horizon, sample_z=True)
    loss = rollout_loss(poses, bodies, batch)
    kl = 0.5 * (mu**2 + logvar.exp() - 1 - logvar).sum(-1).mean()
    priv = F.mse_loss(model.privileged_head(mu), batch["privileged"])
    return loss + beta * kl + privileged_weight * priv, {"rollout": loss.item(), "kl": kl.item(), "privileged": priv.item()}


def context_length(rng):
    """0.5–30 s, log-uniform; 10 % of samples get no context (the average robot)."""
    if rng.random() < 0.1:
        return 0
    return int(math.exp(rng.uniform(math.log(30), math.log(1800))))


# --- training -------------------------------------------------------------


def cmd_train(args):
    device = torch.device(args.device)
    torch.manual_seed(args.seed)
    rng = np.random.default_rng(args.seed)
    with open(args.prepared, "rb") as f:
        data = pickle.load(f)
    train, val = Population(data["train"]), Population(data["val"])
    print(f"{len(train.robots)} training robots, {len(val.robots)} validation robots", flush=True)

    model = AdaptiveTwin().to(device)
    print("parameters:", sum(p.numel() for p in model.parameters()), flush=True)
    opt = torch.optim.AdamW(model.parameters(), lr=args.lr, weight_decay=1e-4)
    n = args.iters
    schedule = [(10, n // 8), (30, n // 8), (60, n // 4), (120, n // 2)]
    total = sum(k for _, k in schedule)
    sched = torch.optim.lr_scheduler.LambdaLR(opt, lambda i: 0.5 * (1 + math.cos(math.pi * min(i, total) / total)))

    vrng = np.random.default_rng(1234)  # fixed validation set: 30 s contexts, 2 s targets
    val_batches = [val.sample(args.batch, 120, 1800, vrng, device) for _ in range(4)]

    def validate():
        model.eval()
        with torch.no_grad():
            losses = [rollout_loss(*run(model, b, 120)[:2], b).item() for b in val_batches]
        model.train()
        return float(np.mean(losses))

    best, step, start = float("inf"), 0, time.time()
    args.out.mkdir(parents=True, exist_ok=True)
    for horizon, iters in schedule:
        for _ in range(iters):
            batch = perturb(train.sample(args.batch, horizon, context_length(rng), rng, device), None)
            loss, parts = total_loss(model, batch, horizon)
            opt.zero_grad()
            loss.backward()
            torch.nn.utils.clip_grad_norm_(model.parameters(), 1.0)
            opt.step()
            sched.step()
            step += 1
            if step % args.val_every == 0 or step == total:
                v = validate()
                tag = ""
                if v < best:
                    best = v
                    torch.save({"kind": "adaptive", "state": model.state_dict(), "step": step, "val": v}, args.out / "adaptive.pt")
                    tag = " *"
                print(f"step {step:5d} h={horizon:3d} loss {parts['rollout']:7.3f} kl {parts['kl']:6.2f} "
                      f"priv {parts['privileged']:.3f} val(2 s, 30 s ctx) {v:7.3f}{tag} [{time.time() - start:5.0f} s]", flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="cmd", required=True)
    p = sub.add_parser("prepare")
    p.add_argument("--data", type=Path, default=Path("data/population"))
    p.add_argument("--prepared", type=Path, default=Path("data/population.pkl"))
    t = sub.add_parser("train")
    t.add_argument("--prepared", type=Path, default=Path("data/population.pkl"))
    t.add_argument("--out", type=Path, default=Path("checkpoints"))
    t.add_argument("--device", default="cuda" if torch.cuda.is_available() else "cpu")
    t.add_argument("--iters", type=int, default=6000)
    t.add_argument("--batch", type=int, default=128)
    t.add_argument("--lr", type=float, default=2e-3)
    t.add_argument("--val-every", type=int, default=250)
    t.add_argument("--seed", type=int, default=0)
    args = parser.parse_args()
    {"prepare": cmd_prepare, "train": cmd_train}[args.cmd](args)


if __name__ == "__main__":
    main()
