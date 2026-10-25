"""Computes every metric of the session and saves them to results/.

    python -m nn_sim.evaluate

results/eval.json      one-step, rollout, regimes, physical properties, generalization,
                       data efficiency, ensemble, cost
results/curves.npz     rollout error curves, step responses, sample trajectories
"""

import json
import time

import numpy as np
import torch
from scipy.stats import spearmanr

from . import data, reference, regimes, rollout
from .models import ConstantVelocity, n_params
from .train import DEVICE, RESULTS, load_model

SPLITS = ["test_id", "test_waypoint"]
HORIZONS_S = [0.1, 0.5, 1.0, 2.0, 5.0]
ROLLOUT_TICKS = 300
STARTS = range(0, 301, 60)
ERR_NAMES = ["pos_m", "theta_rad", "vel_mps", "omega_radps"]


def predict(model, z):
    with torch.no_grad():
        return model(torch.as_tensor(z, device=DEVICE)).double().cpu().numpy()


def one_step(pred, d, d_std):
    err = pred - d
    rmse = np.sqrt((err**2).mean(0))
    r2 = 1 - (err**2).sum(0) / ((d - d.mean(0)) ** 2).sum(0)
    nmse = float(((err / d_std) ** 2).mean())
    return dict(rmse=rmse.tolist(), r2=r2.tolist(), nmse=nmse)


def rollout_windows(ds):
    ep = ds.episodes()
    s, u = data.states(ep), data.commands(ep)[:, :-1]
    st = np.concatenate([s[:, k : k + ROLLOUT_TICKS + 1] for k in STARTS])
    us = np.concatenate([u[:, k : k + ROLLOUT_TICKS] for k in STARTS])
    st = st.copy()
    st[..., :2] -= st[:, :1, :2]
    # Unwrap θ so heading errors do not jump at ±π.
    st[..., 2] = np.unwrap(st[..., 2], axis=1)
    return torch.as_tensor(st, device=DEVICE), torch.as_tensor(us, device=DEVICE)


def rollout_errors(model, st, us):
    with torch.no_grad():
        pred = rollout.rollout(model, st[:, 0], us)
        return rollout.errors(pred, st).cpu().numpy()  # (B, T+1, 4)


def summarize_rollout(e):
    out = {}
    for h in HORIZONS_S:
        k = int(round(h / data.DT))
        out[f"{h}s"] = dict(
            median=np.median(e[:, k], 0).tolist(), p95=np.percentile(e[:, k], 95, 0).tolist(), mean=e[:, k].mean(0).tolist()
        )
    # Growth of the median position error with horizon: err ∝ t^p (fit 0.1 s .. 5 s).
    t = np.arange(e.shape[1]) * data.DT
    med = np.median(e[:, :, 0], 0)
    sel = (t >= 0.1) & (med > 0)
    out["growth_exponent_pos"] = float(np.polyfit(np.log(t[sel]), np.log(med[sel]), 1)[0])
    return out, np.median(e, 0), np.percentile(e, 95, 0)


def by_regime(pred, t):
    reg, tags, _ = regimes.label(t["s0"], t["u"])
    groups = {name: reg == i for i, name in enumerate(regimes.TRANSLATION)}
    groups.update(tags)
    groups["all"] = np.ones(len(reg), bool)
    err = pred - t["d"]
    out = {}
    for name, m in groups.items():
        if m.sum() == 0:
            out[name] = dict(n=0, frac=0.0)
            continue
        out[name] = dict(
            n=int(m.sum()),
            frac=float(m.mean()),
            vel_rmse=float(np.sqrt((err[m, 0] ** 2 + err[m, 1] ** 2).mean())),
            omega_rmse=float(np.sqrt((err[m, 2] ** 2).mean())),
            pos_rmse=float(np.sqrt((err[m, 3] ** 2 + err[m, 4] ** 2).mean())),
        )
    return out


# ─── Physical properties from step responses ────────────────────────────────

TRANS_CMDS = [0.5, 1.0, 2.0, 3.0, 3.5, 4.5]
ROT_CMDS = [1.0, 3.0, 6.0, 10.0, 13.0]


def response(model, s0, u, ticks):
    s0 = np.asarray(s0, float)
    us = np.tile(np.asarray(u, float), (ticks, 1))
    true = reference.rollout(s0, us)
    with torch.no_grad():
        pred = rollout.rollout(model, torch.as_tensor(s0[None], device=DEVICE), torch.as_tensor(us[None], device=DEVICE))
    return pred[0].cpu().numpy(), true


def response_metrics(sig, target):
    t = np.arange(len(sig)) * data.DT
    reach = np.nonzero(sig >= 0.9 * target)[0]
    return dict(
        t90=float(t[reach[0]]) if len(reach) else None,
        overshoot=float(max(0.0, sig.max() - target) / target),
        steady=float(sig[-15:].mean()),
        max_rate=float(np.abs(np.diff(sig)).max() / data.DT),
    )


def physical(model):
    p = reference.DEFAULT
    out, curves = dict(translation={}, rotation={}), {}
    for c in TRANS_CMDS:
        pred, true = response(model, np.zeros(6), [c, 0, 0], 90)
        target = min(c, p["max_speed"])
        out["translation"][str(c)] = dict(
            nn=response_metrics(np.hypot(pred[:, 3], pred[:, 4]), target),
            sim=response_metrics(np.hypot(true[:, 3], true[:, 4]), target),
        )
        curves[f"trans_{c}"] = np.stack([np.hypot(pred[:, 3], pred[:, 4]), np.hypot(true[:, 3], true[:, 4])])
    for c in ROT_CMDS:
        pred, true = response(model, np.zeros(6), [0, 0, c], 45)
        target = min(c, p["max_omega"])
        out["rotation"][str(c)] = dict(nn=response_metrics(pred[:, 5], target), sim=response_metrics(true[:, 5], target))
        curves[f"rot_{c}"] = np.stack([pred[:, 5], true[:, 5]])
    # Braking from 3 m/s forward: stopping time and distance.
    pred, true = response(model, np.array([0, 0, 0, 3.0, 0, 0]), [0, 0, 0], 60)

    def stop(s):
        sp = np.hypot(s[:, 3], s[:, 4])
        k = np.nonzero(sp < 0.05)[0]
        k = k[0] if len(k) else len(sp) - 1
        return dict(time=float(k * data.DT), distance=float(s[k, 0]))

    out["braking_from_3mps"] = dict(nn=stop(pred), sim=stop(true))
    curves["brake"] = np.stack([np.hypot(pred[:, 3], pred[:, 4]), np.hypot(true[:, 3], true[:, 4])])
    # Driving in an arc: forward 2 m/s while turning at 3 rad/s, 2 s.
    pred, true = response(model, np.zeros(6), [2.0, 0, 3.0], 120)
    curves["arc"] = np.stack([pred[:, :2], true[:, :2]])
    out["arc_final_pos_error_m"] = float(np.hypot(*(pred[-1, :2] - true[-1, :2])))
    # Effective limits estimated from the NN: biggest commands from rest.
    ttop, rtop = out["translation"]["4.5"]["nn"], out["rotation"]["13.0"]["nn"]
    out["effective_limits"] = dict(
        nn=dict(max_speed=ttop["steady"], max_accel=ttop["max_rate"], max_omega=rtop["steady"], max_alpha=rtop["max_rate"]),
        sim={k: p[k] for k in ("max_speed", "max_accel", "max_omega", "max_alpha")},
    )
    return out, curves


# ─── Ensemble, cost ─────────────────────────────────────────────────────────


def ensemble(models, t, d_std):
    preds = np.stack([predict(m, t["z"]) for m in models]) / d_std  # normalized
    mean, std = preds.mean(0), preds.std(0)
    true = t["d"] / d_std
    err = np.sqrt(((mean - true) ** 2).mean(1))
    sigma = np.sqrt((std**2).mean(1))
    edges = np.quantile(sigma, np.linspace(0, 1, 11))
    bins = np.clip(np.searchsorted(edges, sigma, side="right") - 1, 0, 9)
    return dict(
        nmse_mean=float((err**2).mean()),
        nmse_members=[float((((p - true) ** 2).mean())) for p in preds],
        spearman=float(spearmanr(sigma, err).statistic),
        bin_sigma=[float(sigma[bins == b].mean()) for b in range(10)],
        bin_err=[float(err[bins == b].mean()) for b in range(10)],
    )


def timing(model):
    out = {}
    cpu = model.to("cpu").eval()
    torch.set_num_threads(1)
    z = torch.zeros(1, 6, dtype=torch.float32)
    with torch.no_grad():
        for _ in range(200):
            cpu(z)
        t0 = time.perf_counter()
        for _ in range(5000):
            cpu(z)
        out["torch_cpu_batch1_us"] = (time.perf_counter() - t0) / 5000 * 1e6
    torch.set_num_threads(torch.get_num_interop_threads())
    try:
        import onnxruntime as ort

        path = RESULTS / "m1.onnx"
        torch.onnx.export(cpu, (z,), str(path), input_names=["z"], output_names=["d"], dynamo=False)
        opts = ort.SessionOptions()
        opts.intra_op_num_threads = 1
        sess = ort.InferenceSession(str(path), opts, providers=["CPUExecutionProvider"])
        zn = np.zeros((1, 6), np.float32)
        for _ in range(200):
            sess.run(None, {"z": zn})
        t0 = time.perf_counter()
        for _ in range(5000):
            sess.run(None, {"z": zn})
        out["onnx_cpu_batch1_us"] = (time.perf_counter() - t0) / 5000 * 1e6
        path.unlink()
    except Exception as e:  # noqa: BLE001 - timing is optional
        out["onnx_error"] = str(e)
    if DEVICE == "cuda":
        gpu = model.to(DEVICE)
        zb = torch.zeros(4096, 6, device=DEVICE)
        with torch.no_grad():
            for _ in range(50):
                gpu(zb)
            torch.cuda.synchronize()
            t0 = time.perf_counter()
            for _ in range(500):
                gpu(zb)
            torch.cuda.synchronize()
        out["torch_gpu_batch4096_us_per_sample"] = (time.perf_counter() - t0) / 500 / 4096 * 1e6
    model.to(DEVICE)
    return out


def main():
    train_t = data.transitions(data.load("train"))
    d_std = train_t["d"].std(0)
    splits = {name: data.load(name) for name in SPLITS}
    trans = {name: data.transitions(ds) for name, ds in splits.items()}
    wins = {name: rollout_windows(ds) for name, ds in splits.items()}

    models = {"B1": ConstantVelocity().to(DEVICE), "B2": load_model("B2"), "M1": load_model("M1_s0"), "M2": load_model("M2")}
    res = dict(one_step={}, rollout={}, regimes={}, physical={}, generalization={}, data_efficiency={}, cost={})
    curves = {}

    for name, model in models.items():
        print("evaluating", name)
        res["one_step"][name], res["rollout"][name], res["regimes"][name] = {}, {}, {}
        for split in SPLITS:
            pred = predict(model, trans[split]["z"])
            res["one_step"][name][split] = one_step(pred, trans[split]["d"], d_std)
            res["regimes"][name][split] = by_regime(pred, trans[split])
            e = rollout_errors(model, *wins[split])
            res["rollout"][name][split], med, p95 = summarize_rollout(e)
            curves[f"roll_med_{name}_{split}"], curves[f"roll_p95_{name}_{split}"] = med, p95
        if name != "B1":
            res["physical"][name], c = physical(model)
            curves.update({f"phys_{name}_{k}": v for k, v in c.items()})
        g = res["rollout"][name]
        res["generalization"][name] = dict(
            one_step_nmse_ratio=res["one_step"][name]["test_waypoint"]["nmse"] / res["one_step"][name]["test_id"]["nmse"],
            pos_1s_median_ratio=g["test_waypoint"]["1.0s"]["median"][0] / max(g["test_id"]["1.0s"]["median"][0], 1e-12),
        )

    # Sample trajectories for plots: 3 waypoint windows, 5 s.
    st, us = wins["test_waypoint"]
    pick = torch.tensor([0, 7, 21], device=DEVICE)
    with torch.no_grad():
        for name in ("B2", "M1", "M2"):
            curves[f"traj_{name}"] = rollout.rollout(models[name], st[pick, 0], us[pick]).cpu().numpy()
    curves["traj_true"] = st[pick].cpu().numpy()

    print("data efficiency")
    for frac, ck in [("1pct", "M1_1pct"), ("10pct", "M1_10pct"), ("100pct", "M1_s0")]:
        m = load_model(ck)
        info = json.loads((RESULTS / f"train_{ck}.json").read_text())
        res["data_efficiency"][frac] = dict(
            episodes={"1pct": 20, "10pct": 200, "100pct": 2000}[frac],
            **{
                split: dict(
                    nmse=one_step(predict(m, trans[split]["z"]), trans[split]["d"], d_std)["nmse"],
                    pos_1s_median=float(np.median(rollout_errors(m, *wins[split])[:, 60, 0])),
                )
                for split in SPLITS
            },
            train_seconds=info["seconds"],
        )
    n = np.array([res["data_efficiency"][f]["episodes"] for f in ("1pct", "10pct", "100pct")], float)
    for split in SPLITS:
        y = np.array([res["data_efficiency"][f][split]["nmse"] for f in ("1pct", "10pct", "100pct")])
        res["data_efficiency"][f"beta_{split}"] = float(-np.polyfit(np.log(n), np.log(y), 1)[0])

    print("ensemble")
    members = [load_model(f"M1_s{s}") for s in range(5)]
    res["ensemble"] = {split: ensemble(members, trans[split], d_std) for split in SPLITS}

    print("cost")
    meta = splits["test_id"].meta
    res["cost"] = dict(
        sim_us_per_step=meta["sim_us_per_step"],
        models={
            name: dict(
                params=n_params(models[name]),
                train_seconds=json.loads((RESULTS / f"train_{ck}.json").read_text())["seconds"],
            )
            for name, ck in [("B2", "B2"), ("M1", "M1_s0"), ("M2", "M2")]
        },
        m1_inference=timing(load_model("M1_s0")),
    )
    res["cost"]["models"]["M2"]["train_seconds_total"] = (
        res["cost"]["models"]["M2"]["train_seconds"] + res["cost"]["models"]["M1"]["train_seconds"]
    )

    (RESULTS / "eval.json").write_text(json.dumps(res, indent=1))
    np.savez_compressed(RESULTS / "curves.npz", **curves)
    print("saved", RESULTS / "eval.json")


if __name__ == "__main__":
    main()
