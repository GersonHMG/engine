"""Faulty robots: data, fault-specific models and evaluation.

    python -m nn_sim.fault_study --data      # generate data/faults/*.npy with engine-datagen
    python -m nn_sim.fault_study --train     # retrain / fine-tune on the TRAINED faults
    python -m nn_sim.fault_study --eval      # results/faults.json, results/faults_curves.npz

Datasets per fault: test_mix (seed 3) and test_wp (seed 4), the same seeds as the healthy
test_id / test_waypoint, so the mixed-signal commands are identical to the healthy ones
(paired comparison). The TRAINED faults also get train (seed 1) and val (seed 2).
"""

import argparse
import json
import subprocess
from pathlib import Path

import numpy as np
import torch

from . import data, faults, reference
from .evaluate import HORIZONS_S, one_step, predict, rollout_errors, rollout_windows, summarize_rollout
from .train import ONE_STEP, RESULTS, load_model, save, stats, tensors, train_one_step

ROOT = Path(__file__).resolve().parents[1]
FAULT_DIR = data.DATA_DIR / "faults"
DATAGEN = ROOT.parent / "simulator" / "target" / "release" / "engine-datagen"

SPLITS = {"test_mix": ("mix", 200, 3), "test_wp": ("waypoint", 200, 4), "train": ("mix", 2000, 1), "val": ("mix", 200, 2)}
# Fault-specific models: (variant, training episodes, start from healthy M1?, config)
VARIANTS = [
    ("scratch_100pct", 2000, False, ONE_STEP),
    ("scratch_1pct", 20, False, ONE_STEP),
    ("finetune_1pct", 20, True, dict(ONE_STEP, steps=3000, lr=3e-4)),
    ("finetune_10pct", 200, True, dict(ONE_STEP, steps=3000, lr=3e-4)),
]


def load(fault, split):
    """A faulty dataset; fault "healthy" maps to the session 1 test sets."""
    if fault == "healthy":
        return data.load({"test_mix": "test_id", "test_wp": "test_waypoint", "train": "train", "val": "val"}[split])
    return data.load(f"{fault}_{split}", FAULT_DIR)


def generate():
    FAULT_DIR.mkdir(parents=True, exist_ok=True)
    for name, (_, _, M) in faults.FAULTS.items():
        for split, (mode, episodes, seed) in SPLITS.items():
            if split in ("train", "val") and name not in faults.TRAINED:
                continue
            cmd = [str(DATAGEN), "--mode", mode, "--episodes", str(episodes), "--seconds", "10", "--seed", str(seed),
                   "--fault-name", name, "--fault-matrix", ",".join(repr(float(x)) for x in M.ravel()),
                   "--out", str(FAULT_DIR / f"{name}_{split}")]
            subprocess.run(cmd, check=True)
    # The data must be exactly the simulator's response to M·u.
    for name, (_, _, M) in faults.FAULTS.items():
        t = data.transitions(load(name, "test_mix"))
        pred = reference.step(t["s0"], faults.apply(t["u"], M))
        err = np.abs(pred - t["s1"])
        err[:, 2] = np.abs(data.wrap(pred[:, 2] - t["s1"][:, 2]))
        print(f"{name}: max |reference(M·u) - data| = {err.max():.2e}")


def train():
    st = stats(data.transitions(data.load("train")))  # healthy normalization for every model
    healthy = load_model("M1_s0")
    for name in faults.TRAINED:
        train_ds = load(name, "train")
        val = tensors(data.transitions(load(name, "val")))
        for variant, episodes, finetune, cfg in VARIANTS:
            print(f"{name} {variant}")
            sub = tensors(data.transitions(train_ds.subset(episodes)))
            model, info = train_one_step(st, sub, val, 0, cfg, log=lambda *_: None, init=healthy if finetune else None)
            info["episodes"], info["finetune"] = episodes, finetune
            save(f"fault_{name}_{variant}", model, info, st)
            print(f"  best val NMSE {info['best_val']:.2e} in {info['seconds']:.1f} s")


def evaluate_model(model, ds, d_std):
    t = data.transitions(ds)
    e = rollout_errors(model, *rollout_windows(ds))
    summary, med, p95 = summarize_rollout(e)
    return dict(one_step=one_step(predict(model, t["z"]), t["d"], d_std), rollout=summary), med


def evaluate():
    d_std = data.transitions(data.load("train"))["d"].std(0)
    names = ["healthy"] + list(faults.FAULTS)
    res, curves = dict(healthy_models={}, fault_models={}), {}
    for m in ["M1", "M2"]:
        model = load_model({"M1": "M1_s0", "M2": "M2"}[m])
        res["healthy_models"][m] = {}
        for name in names:
            res["healthy_models"][m][name] = {}
            for split in ("test_mix", "test_wp"):
                r, med = evaluate_model(model, load(name, split), d_std)
                res["healthy_models"][m][name][split] = r
                curves[f"{m}_{name}_{split}"] = med
            print(m, name, "pos 1 s median (mm):",
                  {s: round(res['healthy_models'][m][name][s]['rollout']['1.0s']['median'][0] * 1e3, 1) for s in ("test_mix", "test_wp")})
    for trained in faults.TRAINED:
        kind = faults.FAULTS[trained][0]
        same_kind = ["healthy"] + [n for n, f in faults.FAULTS.items() if f[0] == kind]
        res["fault_models"][trained] = {}
        for variant, *_ in VARIANTS:
            model = load_model(f"fault_{trained}_{variant}")
            info = json.loads((RESULTS / f"train_fault_{trained}_{variant}.json").read_text())
            out = dict(train_seconds=info["seconds"], episodes=info["episodes"], finetune=info["finetune"], tests={})
            for name in same_kind:
                out["tests"][name] = {}
                for split in ("test_mix", "test_wp"):
                    r, med = evaluate_model(model, load(name, split), d_std)
                    out["tests"][name][split] = r
                    if name == trained:
                        curves[f"{trained}_{variant}_{split}"] = med
            res["fault_models"][trained][variant] = out
            print(trained, variant, "pos 1 s median on own fault (mm):",
                  {s: round(out['tests'][trained][s]['rollout']['1.0s']['median'][0] * 1e3, 1) for s in ("test_mix", "test_wp")})
    res["horizons"] = HORIZONS_S
    (RESULTS / "faults.json").write_text(json.dumps(res, indent=1))
    np.savez_compressed(RESULTS / "faults_curves.npz", **curves)


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--data", action="store_true")
    p.add_argument("--train", action="store_true")
    p.add_argument("--eval", action="store_true")
    args = p.parse_args()
    if not (args.data or args.train or args.eval):
        p.error("use --data, --train and/or --eval")
    if args.data:
        generate()
    if args.train:
        train()
    if args.eval:
        evaluate()


if __name__ == "__main__":
    main()
