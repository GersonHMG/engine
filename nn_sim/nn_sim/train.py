"""Trains every model of the session and saves checkpoints + histories.

    python -m nn_sim.train --all          # B2, M1 ensemble (5 seeds), data efficiency, M2
    python -m nn_sim.train --smoke        # M1 overfits 10 episodes (sanity check)
    python -m nn_sim.train --m2           # only M2 (needs the M1_s0 checkpoint)
"""

import argparse
import json
import time
from pathlib import Path

import numpy as np
import torch

from . import data, rollout
from .models import MLP, Linear, n_params

ROOT = Path(__file__).resolve().parents[1]
CKPT = ROOT / "checkpoints"
RESULTS = ROOT / "results"
DEVICE = "cuda" if torch.cuda.is_available() else "cpu"

ONE_STEP = dict(steps=12000, batch=4096, lr=1e-3, eval_every=500)
ROLLOUT = dict(steps=3000, batch=512, horizon=60, lr=3e-4, eval_every=250, one_step_weight=1.0)


def stats(t):
    """Standardization statistics of features and targets."""
    return (t["z"].mean(0), t["z"].std(0), t["d"].mean(0), t["d"].std(0))


def tensors(t):
    return {k: torch.as_tensor(t[k], device=DEVICE) for k in ("z", "d")}


def normalized_mse(model, z, d):
    with torch.no_grad():
        dn = (d.float() - model.d_mean) / model.d_std
        zn = (z.float() - model.z_mean) / model.z_std
        return torch.mean((model.forward_normalized(zn) - dn) ** 2).item()


def train_one_step(st, train, val, seed, cfg=ONE_STEP, log=print, init=None):
    """M1: MLP trained on the one-step loss; keeps the best checkpoint on val.
    `init`: a model to start from (fine-tuning) instead of random weights."""
    torch.manual_seed(seed)
    model = MLP(st).to(DEVICE)
    if init is not None:
        model.load_state_dict(init.state_dict())
    opt = torch.optim.Adam(model.parameters(), lr=cfg["lr"])
    sched = torch.optim.lr_scheduler.CosineAnnealingLR(opt, cfg["steps"], eta_min=cfg["lr"] * 0.01)
    zn = (train["z"].float() - model.z_mean) / model.z_std
    dn = (train["d"].float() - model.d_mean) / model.d_std
    gen = torch.Generator(device=DEVICE).manual_seed(seed)
    history, best, best_state = [], float("inf"), None
    started = time.time()
    for it in range(1, cfg["steps"] + 1):
        idx = torch.randint(0, len(zn), (cfg["batch"],), device=DEVICE, generator=gen)
        loss = torch.mean((model.forward_normalized(zn[idx]) - dn[idx]) ** 2)
        opt.zero_grad(set_to_none=True)
        loss.backward()
        opt.step()
        sched.step()
        if it % cfg["eval_every"] == 0 or it == cfg["steps"]:
            v = normalized_mse(model, val["z"], val["d"])
            history.append(dict(step=it, train=loss.item(), val=v))
            if v < best:
                best, best_state = v, {k: x.clone() for k, x in model.state_dict().items()}
            log(f"  step {it:6d} train {loss.item():.3e} val {v:.3e}")
    model.load_state_dict(best_state)
    return model, dict(history=history, best_val=best, seconds=time.time() - started)


def windows(ds, horizon):
    """All episode windows as tensors: states (E, T+1, 6), commands (E, T, 3)."""
    ep = ds.episodes()
    s = torch.as_tensor(data.states(ep), device=DEVICE)
    u = torch.as_tensor(data.commands(ep)[:, :-1], device=DEVICE)
    return s, u


def sample_windows(s, u, batch, horizon, gen):
    e = torch.randint(0, s.shape[0], (batch,), device=DEVICE, generator=gen)
    k = torch.randint(0, u.shape[1] - horizon + 1, (batch,), device=DEVICE, generator=gen)
    ar = torch.arange(horizon + 1, device=DEVICE)
    st = s[e[:, None], k[:, None] + ar]  # (B, H+1, 6)
    us = u[e[:, None], k[:, None] + ar[:-1]]
    # Translation invariance: start every window at the origin.
    # Not in place: the right-hand side is a view of the same tensor.
    origin = torch.zeros_like(st)
    origin[..., :2] = st[:, :1, :2]
    return st - origin, us


def rollout_val(model, s, u, horizon, gen):
    with torch.no_grad():
        st, us = sample_windows(s, u, 2048, horizon, gen)
        return rollout.loss(rollout.rollout(model, st[:, 0], us), st).item()


def train_rollout(model, train_ds, val_ds, one_step_train, seed, cfg=ROLLOUT, log=print):
    """M2: fine-tunes a one-step model on open-loop rollouts (BPTT over `horizon` ticks)."""
    torch.manual_seed(seed)
    opt = torch.optim.Adam(model.parameters(), lr=cfg["lr"])
    sched = torch.optim.lr_scheduler.CosineAnnealingLR(opt, cfg["steps"], eta_min=cfg["lr"] * 0.01)
    s, u = windows(train_ds, cfg["horizon"])
    vs, vu = windows(val_ds, cfg["horizon"])
    zn = (one_step_train["z"].float() - model.z_mean) / model.z_std
    dn = (one_step_train["d"].float() - model.d_mean) / model.d_std
    gen = torch.Generator(device=DEVICE).manual_seed(seed)
    history, best, best_state = [], float("inf"), None
    best = rollout_val(model, vs, vu, cfg["horizon"], torch.Generator(device=DEVICE).manual_seed(123))
    best_state = {k: x.clone() for k, x in model.state_dict().items()}
    history.append(dict(step=0, train=None, val=best))
    started = time.time()
    for it in range(1, cfg["steps"] + 1):
        st, us = sample_windows(s, u, cfg["batch"], cfg["horizon"], gen)
        # The rollout in float64 states, model in float32.
        loss = rollout.loss(rollout.rollout(model, st[:, 0], us), st)
        idx = torch.randint(0, len(zn), (4096,), device=DEVICE, generator=gen)
        one = torch.mean((model.forward_normalized(zn[idx]) - dn[idx]) ** 2)
        total = loss + cfg["one_step_weight"] * one
        opt.zero_grad(set_to_none=True)
        total.backward()
        torch.nn.utils.clip_grad_norm_(model.parameters(), 1.0)
        opt.step()
        sched.step()
        if it % cfg["eval_every"] == 0 or it == cfg["steps"]:
            v = rollout_val(model, vs, vu, cfg["horizon"], torch.Generator(device=DEVICE).manual_seed(123))
            history.append(dict(step=it, train=loss.item(), val=v, one_step=one.item()))
            if v < best:
                best, best_state = v, {k: x.clone() for k, x in model.state_dict().items()}
            log(f"  step {it:6d} rollout {loss.item():.3e} one-step {one.item():.3e} val-rollout {v:.3e}")
    model.load_state_dict(best_state)
    return model, dict(history=history, best_val=best, seconds=time.time() - started)


def save(name, model, info, st):
    CKPT.mkdir(exist_ok=True)
    torch.save(dict(kind=type(model).__name__, state=model.state_dict(), stats=[np.asarray(x) for x in st]), CKPT / f"{name}.pt")
    info = dict(info, params=n_params(model))
    (RESULTS / f"train_{name}.json").write_text(json.dumps(info, indent=1))


def load_model(name):
    ck = torch.load(CKPT / f"{name}.pt", weights_only=False)
    cls = {"MLP": MLP, "Linear": Linear}[ck["kind"]]
    model = cls(ck["stats"])
    model.load_state_dict(ck["state"])
    return model.to(DEVICE).eval()


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--all", action="store_true")
    p.add_argument("--smoke", action="store_true")
    p.add_argument("--m2", action="store_true")
    args = p.parse_args()
    RESULTS.mkdir(exist_ok=True)

    train_ds, val_ds = data.load("train"), data.load("val")
    t_train = data.transitions(train_ds)
    st = stats(t_train)
    train, val = tensors(t_train), tensors(data.transitions(val_ds))

    if args.smoke:
        small = tensors(data.transitions(train_ds.subset(10)))
        _, info = train_one_step(st, small, small, 0, dict(ONE_STEP, steps=3000, batch=1024))
        print(f"smoke: final train-set normalized MSE {info['best_val']:.2e}")
        (RESULTS / "smoke.json").write_text(json.dumps(info, indent=1))
        return

    if not (args.all or args.m2):
        p.error("use --all, --m2 or --smoke")

    if args.all:
        train_baselines_and_m1(st, train_ds, train, val)

    print("M2 (rollout fine-tune of M1 seed 0)")
    model = load_model("M1_s0").train()
    model, info = train_rollout(model, train_ds, val_ds, train, 0)
    save("M2", model, info, st)


def train_baselines_and_m1(st, train_ds, train, val):
    print("B2 linear")
    t0 = time.time()
    lin = Linear(st).to(DEVICE).fit(train["z"], train["d"])
    save("B2", lin, dict(best_val=normalized_mse(lin, val["z"], val["d"]), seconds=time.time() - t0), st)

    for seed in range(5):
        print(f"M1 seed {seed}")
        model, info = train_one_step(st, train, val, seed)
        save(f"M1_s{seed}", model, info, st)

    for frac, n in [("1pct", 20), ("10pct", 200)]:
        print(f"M1 data {frac}")
        sub = tensors(data.transitions(train_ds.subset(n)))
        model, info = train_one_step(st, sub, val, 0)
        save(f"M1_{frac}", model, info, st)


if __name__ == "__main__":
    main()
