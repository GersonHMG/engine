"""Evaluate the context-adaptive twin on held-out robots
(docs/digital-twin-generalization.md §5).

    python -m twin.evaluate_adaptive --device cpu

Every test robot is new to the model. For each one the twin gets 0, 1, 5,
30 or 120 s of its calibration drive as context, then predicts the robot's
two separate test sessions. Also: test-time adaptation of z, a rolling
context for the wheel that weakens during the session, a per-robot fitted
physics baseline, maneuvers and closed-loop legs against the real dynamics.
"""

import argparse
import json
import pickle
from pathlib import Path

import numpy as np
import torch

from .adaptive import TRANSITION_DIM, AdaptiveTwin, rollout
from .evaluate import LEGS, MANEUVERS, heading_err, settle_time
from .excitation import go_to
from .models import MAX_DELAY, Prior, PriorModel
from .models import rollout as prior_rollout
from .population import Batch, PopulationSim
from .robots import TICK, quantize
from .adaptive import transition_features
from .train_adaptive import PREFIX, STILL_TICKS, WARM, Session, collate, rollout_loss, target_window

CONTEXTS_S = (0, 1, 5, 30, 120)
HORIZON = 120
IN_RANGE = ("perfect", "bad_wheel", "lateral", "two_bad_wheels", "bad_wheel_lateral", "heavy_bad_wheel")
FAULTY = ("bad_wheel", "lateral", "two_bad_wheels", "bad_wheel_lateral", "heavy_bad_wheel")
CRITERIA = {
    "pos_1s_30s_ctx_m": 0.03,  # in-range robots, 30 s of context
    "context_gain": 0.6,  # faulty robots: error with 30 s ≤ 0.6 × error with none
    "vs_fitted_physics": 0.5,  # ≥ 50 % better than ssl-sim fitted to the same 120 s
}


def calibration_context(sess: Session, seconds: float) -> np.ndarray:
    start = int(np.searchsorted(sess.idx, STILL_TICKS))
    n = int(seconds / TICK)
    ks = sess.idx[start : start + n]
    return transition_features(sess.cmd, sess.body, ks) if len(ks) else np.zeros((0, TRANSITION_DIM), np.float32)


def windows(sess: Session, every=15, lo_tick=0, hi_tick=None):
    """Regular target windows; positions are counted in the concatenated session."""
    out, offset = [], 0
    for i, g in enumerate(sess.segments):
        L = len(g["cmd"])
        for s in range(WARM + PREFIX, L - HORIZON - 1, every):
            pos = offset + s
            if pos >= lo_tick and (hi_tick is None or pos < hi_tick):
                out.append((i, s, pos))
        offset += L
    return out


@torch.no_grad()
def errors(model, sess: Session, wins, contexts, device, z=None):
    targets = [target_window(sess, i, s, HORIZON) for i, s, _ in wins]
    batch = collate(targets, contexts, device)
    mu, _ = model.encode(batch["context"], batch["mask"])
    z = mu if z is None else z.expand(len(wins), -1)
    poses, _ = rollout(model, z, batch["cmd"], batch["v_warm"], batch["pose0"], WARM, HORIZON, PREFIX)
    truth = batch["truth_pose"]
    pos = torch.linalg.norm(poses[..., :2] - truth[..., :2], dim=-1).cpu().numpy()
    head = np.degrees(heading_err(poses[..., 2].cpu().numpy(), truth[..., 2].cpu().numpy()))
    return pos, head, mu


def adapt_z(model, calib: Session, z0, device, steps=150, seconds=30):
    """Test-time adaptation: gradient descent on z only, on the calibration data."""
    wins = windows(calib, every=20, lo_tick=STILL_TICKS, hi_tick=STILL_TICKS + int(seconds / TICK))
    if not wins:
        return z0
    targets = [target_window(calib, i, s, 60) for i, s, _ in wins]
    batch = collate(targets, [np.zeros((0, TRANSITION_DIM), np.float32)] * len(targets), device)
    z = z0.clone().requires_grad_(True)
    opt = torch.optim.Adam([z], lr=0.05)
    model.eval()
    for _ in range(steps):
        poses, bodies = rollout(model, z.expand(len(wins), -1), batch["cmd"], batch["v_warm"], batch["pose0"], WARM, 60, PREFIX)
        loss = rollout_loss(poses, bodies, batch) + 1e-3 * (z**2).sum()
        opt.zero_grad()
        loss.backward()
        opt.step()
    return z.detach()


def fitted_physics(calib: Session, tests: list, device):
    """Baseline: ssl-sim's model with parameters and delay fitted to the same 120 s."""
    wins = windows(calib, every=10, lo_tick=STILL_TICKS)
    targets = [target_window(calib, i, s, 30) for i, s, _ in wins]
    empty = [np.zeros((0, TRANSITION_DIM), np.float32)] * len(targets)
    batch = collate(targets, empty, device)
    robot = torch.zeros(len(targets), dtype=torch.long, device=device)
    best = None
    for d in range(2, 12):
        model = PriorModel(Prior(1, [d])).to(device)
        opt = torch.optim.Adam(model.parameters(), lr=3e-2)
        for _ in range(150):
            poses, bodies = prior_rollout(model, robot, batch["cmd"], batch["v_warm"], batch["pose0"], WARM, 30)
            loss = rollout_loss(poses, bodies, batch)
            opt.zero_grad()
            loss.backward()
            opt.step()
        if best is None or loss.item() < best[0]:
            best = (loss.item(), model)
    model = best[1]
    pos_all = []
    with torch.no_grad():
        for sess in tests:
            wins = windows(sess)
            tb = collate([target_window(sess, i, s, HORIZON) for i, s, _ in wins], [np.zeros((0, TRANSITION_DIM), np.float32)] * len(wins), device)
            r = torch.zeros(len(wins), dtype=torch.long, device=device)
            poses, _ = prior_rollout(model, r, tb["cmd"], tb["v_warm"], tb["pose0"], WARM, HORIZON)
            pos_all.append(torch.linalg.norm(poses[..., :2] - tb["truth_pose"][..., :2], dim=-1).cpu().numpy())
    return np.concatenate(pos_all)


class AdaptiveSim:
    """Step-by-step simulator of one robot from a dynamics code z."""

    def __init__(self, model, z, device):
        self.model, self.z, self.device = model, z, device
        self.params = model.physics(z)

    @torch.no_grad()
    def reset(self, x, y, theta):
        self.hist = torch.zeros(1, PREFIX + 1, 3, device=self.device)
        zeros = torch.zeros(1, WARM, 3, device=self.device)
        self.h = self.model.warm_up(zeros, torch.zeros(1, WARM, PREFIX + 1, 3, device=self.device), self.z, self.params)
        self.pose = np.array([x, y, theta], dtype=float)
        self.v = torch.zeros(1, 3, device=self.device)

    @torch.no_grad()
    def step(self, cmd):
        c = torch.tensor(quantize(np.asarray(cmd, float)), dtype=torch.float32, device=self.device).reshape(1, 1, 3)
        self.hist = torch.cat([self.hist[:, 1:], c], 1)
        v_next, self.h = self.model.step(self.h, self.v, self.hist, self.z, self.params)
        vn = v_next[0].cpu().numpy().astype(float)
        th = self.pose[2]
        self.pose[:2] += np.array([np.cos(th) * vn[0] - np.sin(th) * vn[1], np.sin(th) * vn[0] + np.cos(th) * vn[1]]) * TICK
        self.pose[2] += vn[2] * TICK
        a = -vn[2] * TICK
        self.v = torch.tensor([[np.cos(a) * vn[0] - np.sin(a) * vn[1], np.sin(a) * vn[0] + np.cos(a) * vn[1], vn[2]]],
                              dtype=torch.float32, device=self.device)
        return self.pose.copy()


def real_sim(params):
    sim = PopulationSim(Batch([params]), duration=1e9)
    return sim


def maneuvers_and_loop(model, z, params, device):
    delay = params["vision_delay"]
    man = []
    for name, cmd in MANEUVERS:
        commands = [cmd] * int(1.5 / TICK) + [(0.0, 0.0, 0.0)] * int(0.5 / TICK)
        sim = real_sim(params)
        truth = [sim.pose[0].copy()]
        for c in commands:
            sim.step(np.array([c], float))
            truth.append(sim.pose[0].copy())
        truth = np.array(truth)
        seen = np.concatenate([np.repeat(truth[:1], delay, 0), truth])  # what vision shows
        twin = AdaptiveSim(model, z, device)
        twin.reset(0.0, 0.0, 0.0)
        pred = np.array([twin.step(c) for c in commands])
        end = seen[len(commands)]  # pred[-1] is after len(commands) commands, as vision sees it
        man.append({"maneuver": name, "end error m": float(np.linalg.norm(pred[-1, :2] - end[:2])),
                    "heading error deg": float(np.degrees(heading_err(pred[-1, 2], end[2])))})
    loop = []
    for start, target, speed in LEGS:
        ticks = int(6.0 / TICK)
        tgt, ms = np.array([target]), np.array([speed])
        sim = real_sim(params)
        sim.reset(np.array([start]))
        hist = [sim.pose[0].copy()] * (delay + 1)
        real = []
        for _ in range(ticks):
            p = hist[-1 - delay]
            real.append(p)
            sim.step(go_to(p[None], tgt, ms))
            hist.append(sim.pose[0].copy())
        twin = AdaptiveSim(model, z, device)
        twin.reset(*start)
        model_path, p = [], np.array(start, float)
        for _ in range(ticks):
            model_path.append(p.copy())
            p = twin.step(go_to(p[None], tgt, ms)[0])
        real, model_path = np.array(real), np.array(model_path)
        loop.append({"leg": f"{start[:2]} -> {target[:2]} @ {speed}", "separation m": float(np.linalg.norm(real[:, :2] - model_path[:, :2], axis=1).max()),
                     "settle diff s": abs(settle_time(real, tgt[0]) - settle_time(model_path, tgt[0]))})
    return man, loop


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--prepared", type=Path, default=Path("data/population.pkl"))
    parser.add_argument("--data", type=Path, default=Path("data/population"))
    parser.add_argument("--checkpoint", type=Path, default=Path("checkpoints/adaptive.pt"))
    parser.add_argument("--out", type=Path, default=Path("reports"))
    parser.add_argument("--device", default="cuda" if torch.cuda.is_available() else "cpu")
    parser.add_argument("--name", default="evaluation_adaptive", help="report file name (without extension)")
    args = parser.parse_args()
    device = torch.device(args.device)

    with open(args.prepared, "rb") as f:
        data = pickle.load(f)
    model = AdaptiveTwin().to(device)
    ckpt = torch.load(args.checkpoint, map_location=device, weights_only=False)
    model.load_state_dict(ckpt["state"])
    model.eval()

    robots = {}
    for d in data["test"]:
        robots.setdefault(d["robot"], {})[d["session"]] = Session(d)

    report, lines = {"criteria": CRITERIA, "robots": {}}, []
    lines += ["# Context-adaptive twin on held-out robots", "",
              f"Checkpoint step {ckpt['step']}. None of these robots was in training. Errors are against the "
              "noise-free truth on two test sessions separate from the calibration drive.", "",
              "| robot | " + " | ".join(f"{c} s ctx" for c in CONTEXTS_S) + " | 30 s + TTA | oracle z (true hardware) | fitted physics (120 s) | pos 2 s (30 s ctx) | heading 1 s (30 s ctx) |",
              "|---|" + "---|" * (len(CONTEXTS_S) + 6)]
    for name in sorted(robots):
        sess = robots[name]
        calib, tests = sess["calibration"], [sess["test0"], sess["test1"]]
        r = {"pos_1s_by_context": {}}
        for c in CONTEXTS_S:
            ctx = calibration_context(calib, c)
            pos = np.concatenate([errors(model, t, windows(t), [ctx] * len(windows(t)), device)[0] for t in tests])
            r["pos_1s_by_context"][c] = float(np.median(pos[:, 59]))
            if c == 30:
                r["pos_2s_30"] = float(np.median(pos[:, 119]))
                heads = np.concatenate([errors(model, t, windows(t), [ctx] * len(windows(t)), device)[1] for t in tests])
                r["heading_1s_30"] = float(np.median(heads[:, 59]))
                ctx30 = ctx
        with torch.no_grad():
            mu30, logvar30 = model.encode(torch.from_numpy(ctx30[None]).to(device), torch.ones(1, len(ctx30), dtype=torch.bool, device=device))
        r["z_std_30"] = float(torch.exp(0.5 * logvar30).mean())
        with torch.no_grad():
            z_oracle = model.teacher(torch.from_numpy(calib.privileged[None]).to(device))
        pos = np.concatenate([errors(model, t, windows(t), [ctx30] * len(windows(t)), device, z=z_oracle)[0] for t in tests])
        r["pos_1s_oracle"] = float(np.median(pos[:, 59]))
        z_tta = adapt_z(model, calib, mu30, device)
        pos = np.concatenate([errors(model, t, windows(t), [ctx30] * len(windows(t)), device, z=z_tta)[0] for t in tests])
        r["pos_1s_tta"] = float(np.median(pos[:, 59]))
        r["pos_1s_fitted_physics"] = float(np.median(fitted_physics(calib, tests, device)[:, 59]))
        if name == "degrading_wheel":
            # Rolling context: the previous 30 s of the same session, vs. the healthy calibration.
            roll, fixed = [], []
            for t in tests:
                wins = [w for w in windows(t) if w[2] >= 1800 + STILL_TICKS]
                late = [w for w in wins if w[2] >= len(t.cmd) // 2]
                ctxs = []
                for _, _, pos_t in late:
                    j = int(np.searchsorted(t.idx, pos_t))
                    ks = t.idx[max(0, j - 1800) : j]
                    ctxs.append(transition_features(t.cmd, t.body, ks))
                roll.append(errors(model, t, late, ctxs, device)[0])
                fixed.append(errors(model, t, late, [ctx30] * len(late), device)[0])
            r["second_half_rolling"] = float(np.median(np.concatenate(roll)[:, 59]))
            r["second_half_calibration"] = float(np.median(np.concatenate(fixed)[:, 59]))
        params = json.loads((args.data / "test" / name / "calibration.json").read_text())["params"]
        params["wheel"] = tuple(params["wheel"])
        # From rest, with the estimated z and with the true-hardware z: separates
        # dynamics error from identification error.
        r["maneuvers"], r["closed_loop"] = maneuvers_and_loop(model, mu30, params, device)
        r["maneuvers_oracle"], r["closed_loop_oracle"] = maneuvers_and_loop(model, z_oracle, params, device)
        report["robots"][name] = r
        ctx_cells = " | ".join(f"{r['pos_1s_by_context'][c] * 100:.1f}" for c in CONTEXTS_S)
        lines.append(f"| {name} | {ctx_cells} | {r['pos_1s_tta'] * 100:.1f} | {r['pos_1s_oracle'] * 100:.1f} | {r['pos_1s_fitted_physics'] * 100:.1f} | "
                     f"{r['pos_2s_30'] * 100:.1f} | {r['heading_1s_30']:.2f}° |")
        print(lines[-1], flush=True)

    lines += ["", "Position errors in cm (median, 1 s open-loop unless noted). TTA = test-time adaptation of z on the first 30 s of the calibration drive.", ""]
    d = report["robots"].get("degrading_wheel")
    if d:
        lines += [f"Degrading wheel, second half of the test sessions: rolling 30 s context {d['second_half_rolling'] * 100:.1f} cm "
                  f"vs. healthy calibration {d['second_half_calibration'] * 100:.1f} cm.", ""]
    lines += ["From rest: 10 maneuvers (end error after 2 s) and 6 closed-loop legs (max path separation), "
              "median / worst, in cm. 'estimated' = z from 30 s of calibration, 'true' = z from the true hardware.", "",
              "| robot | maneuvers, estimated z | maneuvers, true z | closed loop, estimated z | closed loop, true z | worst maneuver (estimated z) |",
              "|---|---|---|---|---|---|"]

    def med_worst(rows, key):
        v = np.array([m[key] for m in rows]) * 100
        return f"{np.median(v):.1f} / {v.max():.1f}"

    for name, r in report["robots"].items():
        wm = max(r["maneuvers"], key=lambda m: m["end error m"])
        lines.append(f"| {name} | {med_worst(r['maneuvers'], 'end error m')} | {med_worst(r['maneuvers_oracle'], 'end error m')} | "
                     f"{med_worst(r['closed_loop'], 'separation m')} | {med_worst(r['closed_loop_oracle'], 'separation m')} | "
                     f"{wm['maneuver']}: {wm['end error m'] * 100:.1f} cm, {wm['heading error deg']:.1f}° |")

    checks = {}
    rb = report["robots"]
    checks[f"in-range robots, 30 s context: 1 s error ≤ {CRITERIA['pos_1s_30s_ctx_m'] * 100:.0f} cm"] = all(
        rb[n]["pos_1s_by_context"][30] <= CRITERIA["pos_1s_30s_ctx_m"] for n in IN_RANGE if n in rb)
    checks[f"faulty robots: 30 s context ≤ {CRITERIA['context_gain']} × no context"] = all(
        rb[n]["pos_1s_by_context"][30] <= CRITERIA["context_gain"] * rb[n]["pos_1s_by_context"][0] for n in FAULTY if n in rb)
    checks[f"in-range robots: ≥ {CRITERIA['vs_fitted_physics'] * 100:.0f} % better than fitted physics"] = all(
        rb[n]["pos_1s_by_context"][120] <= (1 - CRITERIA["vs_fitted_physics"]) * rb[n]["pos_1s_fitted_physics"] for n in IN_RANGE if n in rb)
    if d:
        checks["degrading wheel: rolling context beats the stale calibration"] = d["second_half_rolling"] < d["second_half_calibration"]
    report["checks"] = checks
    lines += ["", "| check | result |", "|---|---|"] + [f"| {k} | {'pass' if v else 'FAIL'} |" for k, v in checks.items()]
    args.out.mkdir(parents=True, exist_ok=True)
    (args.out / f"{args.name}.md").write_text("\n".join(lines) + "\n")
    (args.out / f"{args.name}.json").write_text(json.dumps(report, indent=1, ensure_ascii=False))
    print("\n".join(lines[-(len(checks) + 2):]))


if __name__ == "__main__":
    main()
