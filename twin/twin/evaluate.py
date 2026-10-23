"""Evaluate baselines and the twin on held-out sessions and scenario maneuvers (plan §6).

    python -m twin.evaluate --data data/prepared.pkl --checkpoints checkpoints --out reports
"""

import argparse
import json
import pickle
from pathlib import Path

import numpy as np
import torch

from .data import load_split
from .excitation import go_to
from .robots import TICK, RobotSim, scenario
from .simulate import Twin, load_model
from .train import Windows, run

HORIZON = 120  # 2 s
MARKS = {"0.5 s": 29, "1 s": 59, "2 s": 119}
VISION_DELAY = 2  # ticks, of the synthetic vision (generate.Vision)

MODELS = {
    "ssl-sim (default)": "baseline1_sslsim.pt",
    "ssl-sim (fitted)": "baseline2_fitted.pt",
    "ARX (linear)": "baseline3_arx.pt",
    "GRU twin": "twin_gru.pt",
}

# Acceptance criteria, fixed before looking at the results.
CRITERIA = {
    "pos_1s_median_m": 0.03,
    "heading_1s_median_deg": 2.0,
    "improvement_vs_fitted": 0.5,
    "maneuver_pos_m": 0.05,
    "maneuver_heading_deg": 3.0,
    # Free-running a whole 2-min test session on its own predictions.
    "freerun_vel_rmse_ms": 0.10,
    "freerun_omega_rmse_rads": 0.5,
    "freerun_growth": 1.5,  # last-30-s error / first-30-s error
    # Closed loop: the go_to controller drives the twin from the twin's own pose.
    "loop_settle_s": 0.15,
    "loop_path_m": 0.05,
}

# Closed-loop legs: (start x, y, theta) -> (target x, y, theta), max speed.
LEGS = [
    ((-2.0, 0.0, 0.0), (1.0, 0.0, 0.0), 2.0),
    ((0.0, -1.5, 0.0), (0.0, 1.5, 0.0), 2.0),
    ((-1.0, -1.0, 0.0), (1.0, 1.0, 1.57), 2.5),
    ((1.5, 0.5, 3.14), (-1.5, -0.5, 0.0), 3.0),
    ((0.0, 0.0, 0.0), (0.3, 0.2, 0.0), 1.0),
    ((-2.0, 1.0, -1.57), (2.0, -1.0, 1.57), 3.0),
]

# (name, command held 1.5 s, then 0.5 s of zero command)
MANEUVERS = [
    ("forward 1 m/s", (1.0, 0.0, 0.0)),
    ("forward 2.5 m/s", (2.5, 0.0, 0.0)),
    ("backward 1.5 m/s", (-1.5, 0.0, 0.0)),
    ("left 1 m/s", (0.0, 1.0, 0.0)),
    ("right 2 m/s", (0.0, -2.0, 0.0)),
    ("left creep 0.15 m/s", (0.0, 0.15, 0.0)),
    ("forward creep 0.1 m/s", (0.1, 0.0, 0.0)),
    ("diagonal 1.5 m/s", (1.06, 1.06, 0.0)),
    ("spin 4 rad/s", (0.0, 0.0, 4.0)),
    ("forward 1 m/s + spin 3 rad/s", (1.0, 0.0, 3.0)),
]


def heading_err(a, b):
    return np.abs(np.angle(np.exp(1j * (a - b))))


@torch.no_grad()
def rollout_metrics(models, test: Windows, robot_index, robot):
    items = [(i, s) for i, s in test.regular(HORIZON, 30) if test.segments[i].robot == robot]
    batch = test.collate(items, HORIZON)
    truth = batch["truth_pose"].numpy()
    raw = batch["raw"].numpy()
    out = {}
    # Floor: how far the smoothed start pose already is from the truth.
    out["noise floor"] = {
        "start pose error (smoothed vs truth), m": float(np.median(np.linalg.norm(batch["pose0"][:, :2] - batch["truth_pose0"][:, :2], axis=1))),
        "raw vision error vs truth, m": float(np.nanmedian(np.linalg.norm(raw[..., :2] - truth[..., :2], axis=-1))),
    }
    for name, model in models.items():
        poses, bodies = run(model, batch, HORIZON)
        poses, bodies = poses.numpy(), bodies.numpy()
        pos = np.linalg.norm(poses[..., :2] - truth[..., :2], axis=-1)
        head = np.degrees(heading_err(poses[..., 2], truth[..., 2]))
        vel1 = bodies[:, 0] - batch["truth_body"][:, 0].numpy()
        m = {"one-step velocity RMSE vt, vn, ω": [round(float(x), 4) for x in np.sqrt((vel1**2).mean(0))]}
        for label, j in MARKS.items():
            m[f"pos {label} median m"] = float(np.median(pos[:, j]))
            m[f"pos {label} p90 m"] = float(np.percentile(pos[:, j], 90))
            m[f"heading {label} median deg"] = float(np.median(head[:, j]))
        seen = ~np.isnan(raw[:, 59, 0])
        m["pos 1 s median vs raw vision m"] = float(np.median(np.linalg.norm(poses[seen, 59, :2] - raw[seen, 59, :2], axis=-1)))
        out[name] = m
    return out, len(items)


@torch.no_grad()
def residual_structure(model, test: Windows, robot):
    """R² of a linear fit of one-step velocity residuals (vs smoothed data, no
    truth) on the state and command history: ~0 means nothing systematic is left."""
    items = [(i, s) for i, s in test.regular(1, 3) if test.segments[i].robot == robot]
    batch = test.collate(items, 1)
    _, bodies = run(model, batch, 1)
    res = (bodies[:, 0] - batch["body"][:, 0]).numpy()
    X = np.concatenate([batch["v_warm"][:, -1].numpy(), batch["cmd"][:, -13:].reshape(len(items), -1).numpy(), np.ones((len(items), 1))], 1)
    half = len(X) // 2  # fit on one half, score on the other
    W, *_ = np.linalg.lstsq(X[:half], res[:half], rcond=None)
    pred = X[half:] @ W
    r2 = 1 - ((res[half:] - pred) ** 2).sum(0) / ((res[half:] - res[half:].mean(0)) ** 2).sum(0)
    return r2.round(3).tolist()


def maneuvers(path, robot):
    twin = Twin.load(path, robot)
    rows = []
    for name, cmd in MANEUVERS:
        hold, rest = int(1.5 / TICK), int(0.5 / TICK)
        commands = np.array([cmd] * hold + [(0.0, 0.0, 0.0)] * rest)
        sim = RobotSim(scenario(robot), 1)
        sim.reset(np.zeros((1, 3)))
        truth = []
        for c in commands:
            truth.append(sim.pose[0].copy())
            sim.step(c[None])
        truth.append(sim.pose[0].copy())
        truth = np.array(truth)
        # The twin learnt from vision, which sees the pose VISION_DELAY ticks late.
        truth = np.concatenate([np.repeat(truth[:1], VISION_DELAY, 0), truth])[1 : len(commands) + 1]
        pred = twin.rollout((0.0, 0.0, 0.0), commands)
        end_pos = float(np.linalg.norm(pred[-1, :2] - truth[-1, :2]))
        end_head = float(np.degrees(heading_err(pred[-1, 2], truth[-1, 2])))
        rows.append(
            {
                "maneuver": name,
                "real end (x, y, θ°)": [round(float(truth[-1, 0]), 3), round(float(truth[-1, 1]), 3), round(float(np.degrees(truth[-1, 2])), 1)],
                "twin end (x, y, θ°)": [round(float(pred[-1, 0]), 3), round(float(pred[-1, 1]), 3), round(float(np.degrees(pred[-1, 2])), 1)],
                "end position error m": round(end_pos, 4),
                "end heading error deg": round(end_head, 2),
                "max position error m": round(float(np.linalg.norm(pred[:, :2] - truth[:, :2], axis=1).max()), 4),
            }
        )
    return rows


@torch.no_grad()
def free_run(path, data_root, robot):
    """Whole test sessions from rest, the twin fed only commands and its own state."""
    twin = Twin.load(path, robot)
    rows = []
    for session in load_split(data_root, "test", [robot]):
        truth_pose, truth_body = session.truth["pose"], session.truth["body"]
        pred = twin.rollout(tuple(truth_pose[0]), session.cmd[:-1])
        # pred[i] (after command i) is what vision shows at row i + 1.
        err = pred[:, 3:] - truth_body[1:]
        block = int(30 / TICK)
        first, last = err[:block], err[-block:]
        rmse = np.sqrt((err**2).mean(0))
        growth = np.sqrt((last[:, :2] ** 2).mean()) / max(np.sqrt((first[:, :2] ** 2).mean()), 1e-9)
        rows.append(
            {
                "session": session.program,
                "seconds": round(len(err) * TICK, 1),
                "vel RMSE vt, vn (m/s), ω (rad/s)": rmse.round(4).tolist(),
                "growth": round(float(growth), 2),
                "final position drift m": round(float(np.linalg.norm(pred[-1, :2] - truth_pose[-1, :2])), 3),
                "max |v| twin": round(float(np.abs(pred[:, 3:5]).max()), 2),
            }
        )
    return rows


def settle_time(path: np.ndarray, target: np.ndarray, radius: float = 0.05) -> float:
    """Time after which the robot stays within `radius` of the target."""
    outside = np.flatnonzero(np.linalg.norm(path[:, :2] - target[:2], axis=1) > radius)
    return (outside[-1] + 1) * TICK if len(outside) else 0.0


def closed_loop(path, robot, duration=6.0):
    """The engine-style loop: controller -> command -> twin -> pose -> controller,
    for a fixed time. Compared with the same controller driving the reference
    robot through vision."""
    twin = Twin.load(path, robot)
    ticks = int(duration / TICK)
    rows = []
    for start, target, speed in LEGS:
        target_arr = np.array([target])
        max_speed = np.array([speed])

        sim = RobotSim(scenario(robot), 1)
        sim.reset(np.array([start]))
        seen = [sim.pose[0].copy()] * (VISION_DELAY + 1)
        real = []
        for _ in range(ticks):
            pose = seen[-1 - VISION_DELAY]  # the controller sees vision, VISION_DELAY late
            real.append(pose)
            sim.step(go_to(pose[None], target_arr, max_speed))
            seen.append(sim.pose[0].copy())

        twin.reset(*start)
        model = []
        for _ in range(ticks):
            pose = twin.observe()[:3]
            model.append(pose)
            twin.step(go_to(pose[None], target_arr, max_speed)[0])

        real, model = np.array(real), np.array(model)
        rows.append(
            {
                "leg": f"{start[:2]} -> {target[:2]} @ {speed} m/s",
                "real settle s": round(settle_time(real, target_arr[0]), 3),
                "twin settle s": round(settle_time(model, target_arr[0]), 3),
                "max path separation m": round(float(np.linalg.norm(real[:, :2] - model[:, :2], axis=1).max()), 4),
            }
        )
    return rows


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--data", type=Path, default=Path("data/prepared.pkl"))
    parser.add_argument("--checkpoints", type=Path, default=Path("checkpoints"))
    parser.add_argument("--out", type=Path, default=Path("reports"))
    parser.add_argument("--raw", type=Path, default=Path("data/synthetic"), help="session CSVs, for whole-session free runs")
    args = parser.parse_args()

    with open(args.data, "rb") as f:
        data = pickle.load(f)
    models, robots = {}, None
    for name, file in MODELS.items():
        models[name], ckpt = load_model(args.checkpoints / file)
        robots = ckpt["robots"]
    index = {r: i for i, r in enumerate(robots)}
    test = Windows(data["splits"]["test"], index)

    report = {"criteria": CRITERIA, "robots": {}}
    lines = ["# Digital twin evaluation (synthetic robots)", ""]
    lines += ["Errors are against the noise-free truth on held-out test sessions; models only saw noisy vision.", ""]
    all_pass = True
    for robot in robots:
        metrics, n = rollout_metrics(models, test, index, robot)
        r2 = residual_structure(models["GRU twin"], test, robot)
        man = maneuvers(args.checkpoints / MODELS["GRU twin"], robot)
        runs = free_run(args.checkpoints / MODELS["GRU twin"], args.raw, robot)
        loop = closed_loop(args.checkpoints / MODELS["GRU twin"], robot)
        twin, fitted = metrics["GRU twin"], metrics["ssl-sim (fitted)"]
        improvement = 1 - twin["pos 1 s median m"] / fitted["pos 1 s median m"]
        checks = {
            f"1 s median position error ≤ {CRITERIA['pos_1s_median_m'] * 100:.0f} cm": twin["pos 1 s median m"] <= CRITERIA["pos_1s_median_m"],
            f"1 s median heading error ≤ {CRITERIA['heading_1s_median_deg']}°": twin["heading 1 s median deg"] <= CRITERIA["heading_1s_median_deg"],
            f"≥ {CRITERIA['improvement_vs_fitted'] * 100:.0f} % better than fitted ssl-sim at 1 s": improvement >= CRITERIA["improvement_vs_fitted"],
            f"every maneuver ends within {CRITERIA['maneuver_pos_m'] * 100:.0f} cm and {CRITERIA['maneuver_heading_deg']}°": all(
                m["end position error m"] <= CRITERIA["maneuver_pos_m"] and m["end heading error deg"] <= CRITERIA["maneuver_heading_deg"] for m in man
            ),
            f"2-min free runs: vel RMSE ≤ {CRITERIA['freerun_vel_rmse_ms']} m/s, ω ≤ {CRITERIA['freerun_omega_rmse_rads']} rad/s, error growth ≤ {CRITERIA['freerun_growth']}×": all(
                max(r["vel RMSE vt, vn (m/s), ω (rad/s)"][:2]) <= CRITERIA["freerun_vel_rmse_ms"]
                and r["vel RMSE vt, vn (m/s), ω (rad/s)"][2] <= CRITERIA["freerun_omega_rmse_rads"]
                and r["growth"] <= CRITERIA["freerun_growth"]
                for r in runs
            ),
            f"closed loop: settles within {CRITERIA['loop_settle_s']} s of the real robot, paths within {CRITERIA['loop_path_m'] * 100:.0f} cm": all(
                abs(r["real settle s"] - r["twin settle s"]) <= CRITERIA["loop_settle_s"] and r["max path separation m"] <= CRITERIA["loop_path_m"]
                for r in loop
            ),
        }
        passed = all(checks.values())
        all_pass &= passed
        report["robots"][robot] = {"windows": n, "metrics": metrics, "residual_r2": r2, "maneuvers": man, "free_runs": runs, "closed_loop": loop, "checks": checks, "passed": passed}

        lines += [f"## {robot}: {'PASS' if passed else 'FAIL'}", ""]
        lines += [f"{n} open-loop rollouts of 2 s from the test sessions.", ""]
        lines += ["| model | pos 0.5 s | pos 1 s | pos 1 s p90 | pos 2 s | heading 1 s | one-step vel RMSE (vt, vn, ω) |", "|---|---|---|---|---|---|---|"]
        for name in MODELS:
            m = metrics[name]
            lines.append(
                f"| {name} | {m['pos 0.5 s median m'] * 100:.1f} cm | {m['pos 1 s median m'] * 100:.1f} cm | {m['pos 1 s p90 m'] * 100:.1f} cm | "
                f"{m['pos 2 s median m'] * 100:.1f} cm | {m['heading 1 s median deg']:.2f}° | {m['one-step velocity RMSE vt, vn, ω']} |"
            )
        nf = metrics["noise floor"]
        lines += ["", f"Noise floor: raw vision vs truth {nf['raw vision error vs truth, m'] * 1000:.1f} mm; smoothed start pose vs truth {nf['start pose error (smoothed vs truth), m'] * 1000:.1f} mm.", ""]
        lines += [f"Residual structure (R² of one-step residuals explained by state + commands, vt, vn, ω): {r2}", ""]
        lines += ["| maneuver (1.5 s + 0.5 s stop) | real end (x, y, θ°) | twin end (x, y, θ°) | end error | heading error |", "|---|---|---|---|---|"]
        for m in man:
            lines.append(
                f"| {m['maneuver']} | {m['real end (x, y, θ°)']} | {m['twin end (x, y, θ°)']} | {m['end position error m'] * 100:.1f} cm | {m['end heading error deg']:.2f}° |"
            )
        lines += ["", "Free runs: whole test sessions, the twin fed only the logged commands and its own predictions.", ""]
        lines += ["| session | length | vel RMSE vt, vn, ω | error growth (last/first 30 s) | final drift |", "|---|---|---|---|---|"]
        for r in runs:
            lines.append(f"| {r['session']} | {r['seconds']} s | {r['vel RMSE vt, vn (m/s), ω (rad/s)']} | {r['growth']}× | {r['final position drift m'] * 100:.0f} cm |")
        lines += ["", "Closed loop (6 s per leg): go_to controller fed back with the twin's own pose vs. the same controller on the reference robot (through vision).", ""]
        lines += ["| leg | real settles (5 cm) | twin settles | max path separation |", "|---|---|---|---|"]
        for r in loop:
            lines.append(f"| {r['leg']} | {r['real settle s']:.2f} s | {r['twin settle s']:.2f} s | {r['max path separation m'] * 100:.1f} cm |")
        lines += ["", "| check | result |", "|---|---|"]
        lines += [f"| {k} | {'pass' if v else 'FAIL'} |" for k, v in checks.items()]
        lines += [""]

    report["passed"] = all_pass
    args.out.mkdir(parents=True, exist_ok=True)
    (args.out / "evaluation.json").write_text(json.dumps(report, indent=2, ensure_ascii=False))
    (args.out / "evaluation.md").write_text("\n".join(lines))
    print("\n".join(lines))
    print("ALL PASS" if all_pass else "NOT ALL PASSED")


if __name__ == "__main__":
    main()
