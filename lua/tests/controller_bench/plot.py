# /// script
# requires-python = ">=3.10"
# dependencies = ["matplotlib>=3.8"]
# ///
"""Plots of the benchmark's trajectory traces (CB_TRACE): target vs real.

    uv run lua/tests/controller_bench/plot.py <trace_dir> [<out_dir>]

For every `<test>_<robot>.csv` it writes `<test>_<robot>.png`, and for every test an
overlay of all robots `<test>_all.png`. track.py uses the same functions to attach the
plots to the MLflow runs.

One figure per test and robot:
- left, the field: the target path (the setpoints the controller was given, joined by
  straight dashed lines from the start), the real path, the robot's heading every 0.25 s
  (and the target heading at each setpoint), and the ball (vision) with the kick line;
- right, against time: distance to the setpoint, heading vs target heading, and speed.
"""

from __future__ import annotations

import csv
import math
import sys
from pathlib import Path

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402
from matplotlib.figure import Figure  # noqa: E402

REAL = "#2563eb"
TARGET = "#7c3aed"
BALL = "#ea580c"
GRID = "#d4d4d8"
ROBOT_COLORS = ["#2563eb", "#dc2626", "#16a34a", "#9333ea", "#ca8a04"]


def load(path: Path) -> dict[str, list[float | None]]:
    cols: dict[str, list[float | None]] = {}
    with path.open(encoding="utf-8") as f:
        for row in csv.DictReader(f):
            for k, v in row.items():
                cols.setdefault(k, []).append(float(v) if v not in ("", None) else None)
    return cols


def wrap(a: float) -> float:
    return (a + math.pi) % (2 * math.pi) - math.pi


def setpoints(d: dict) -> list[tuple[float, float, float | None, float]]:
    """Distinct consecutive goals: (x, y, theta, time it was first given)."""
    out = []
    for t, x, y, th in zip(d["t"], d.get("goal_x", []), d.get("goal_y", []), d.get("goal_theta", [])):
        if x is None or y is None:
            continue
        if not out or math.hypot(x - out[-1][0], y - out[-1][1]) > 1e-4 or (
                th is not None and out[-1][2] is not None and abs(wrap(th - out[-1][2])) > 1e-3):
            out.append((x, y, th, t))
    return out


def _field(ax, d: dict, color: str = REAL, label: str = "real path", target: bool = True, ball: bool = True):
    xs, ys = d["x"], d["y"]
    sp = setpoints(d)
    if target and sp:
        tx = [xs[0]] + [p[0] for p in sp]
        ty = [ys[0]] + [p[1] for p in sp]
        ax.plot(tx, ty, "--", color=TARGET, lw=1.2, label="target path")
        ax.scatter([p[0] for p in sp], [p[1] for p in sp], marker="x", color=TARGET, s=40, zorder=4,
                   label="setpoints")
        for x, y, th, _ in sp:
            if th is not None:
                ax.annotate("", xy=(x + 0.15 * math.cos(th), y + 0.15 * math.sin(th)), xytext=(x, y),
                            arrowprops=dict(arrowstyle="->", color=TARGET, lw=1.2))
    ax.plot(xs, ys, color=color, lw=1.6, label=label)
    # Heading every 0.25 s.
    step = max(1, int(round(0.25 / max(d["t"][1] - d["t"][0], 1e-3)))) if len(d["t"]) > 1 else 1
    idx = range(0, len(xs), step)
    ax.quiver([xs[i] for i in idx], [ys[i] for i in idx],
              [math.cos(d["theta"][i]) for i in idx], [math.sin(d["theta"][i]) for i in idx],
              color=color, alpha=0.55, scale=28, width=0.004, headwidth=3)
    ax.plot(xs[0], ys[0], "o", color=color, ms=5)
    if ball:
        bx = [v for v in d.get("ball_x", []) if v is not None]
        by = [v for v in d.get("ball_y", []) if v is not None]
        if bx and (max(bx) - min(bx) > 0.05 or max(by) - min(by) > 0.05):
            ax.plot(bx, by, color=BALL, lw=1.4, label="ball (vision)")
            # The kick line: from where the ball started, along the heading it was to be kicked at.
            th = next((p[2] for p in reversed(sp) if p[2] is not None), None)
            if th is not None:
                ax.plot([bx[0], bx[0] + 3.5 * math.cos(th)], [by[0], by[0] + 3.5 * math.sin(th)], ":",
                        color=BALL, lw=1.2, label="kick line")
    ax.set_aspect("equal", adjustable="datalim")
    ax.grid(True, color=GRID, lw=0.6)
    ax.set_xlabel("x (m)")
    ax.set_ylabel("y (m)")


def plot_run(path: Path, title: str | None = None) -> Figure:
    d = load(path)
    fig = plt.figure(figsize=(13, 6.5), layout="constrained")
    gs = fig.add_gridspec(3, 2, width_ratios=[1.25, 1])
    ax = fig.add_subplot(gs[:, 0])
    _field(ax, d)
    ax.legend(loc="best", fontsize=8)
    ax.set_title("field")

    t = d["t"]
    err, th_goal, speed = [], [], []
    for i in range(len(t)):
        gx, gy, gth = d["goal_x"][i], d["goal_y"][i], d["goal_theta"][i]
        err.append(None if gx is None else 1000 * math.hypot(d["x"][i] - gx, d["y"][i] - gy))
        th_goal.append(None if gth is None else math.degrees(gth))
        speed.append(math.hypot(d["vx"][i], d["vy"][i]))
    heading = [math.degrees(v) for v in d["theta"]]

    a1 = fig.add_subplot(gs[0, 1])
    a1.plot(t, [e if e is not None else float("nan") for e in err], color=REAL, lw=1.2)
    a1.set_ylabel("to setpoint (mm)")
    a1.set_yscale("symlog", linthresh=20)
    a1.axhline(20, color=TARGET, lw=0.8, ls=":")
    a2 = fig.add_subplot(gs[1, 1], sharex=a1)
    a2.plot(t, [v if v is not None else float("nan") for v in th_goal], "--", color=TARGET, lw=1.2, label="target")
    a2.plot(t, heading, color=REAL, lw=1.2, label="real")
    a2.set_ylabel("heading (°)")
    a2.legend(loc="best", fontsize=8)
    a3 = fig.add_subplot(gs[2, 1], sharex=a1)
    a3.plot(t, speed, color=REAL, lw=1.2)
    a3.set_ylabel("speed (m/s)")
    a3.set_xlabel("t (s)")
    for a in (a1, a2, a3):
        a.grid(True, color=GRID, lw=0.6)
    fig.suptitle(title or path.stem)
    return fig


def plot_overlay(paths: dict[str, Path], title: str) -> Figure:
    """All robots of one test on one field."""
    fig, ax = plt.subplots(figsize=(8, 7), layout="constrained")
    for i, (robot, path) in enumerate(paths.items()):
        _field(ax, load(path), color=ROBOT_COLORS[i % len(ROBOT_COLORS)], label=robot, target=i == 0, ball=False)
    ax.legend(loc="best", fontsize=8)
    ax.set_title(title)
    return fig


def by_test(trace_dir: Path) -> dict[str, dict[str, Path]]:
    """{test: {robot: csv}} from `<test>_<robot>.csv` names (tests have no robot names in them)."""
    tests = ("go_to_point", "square", "kick_to_point")
    out: dict[str, dict[str, Path]] = {}
    for f in sorted(trace_dir.glob("*.csv")):
        test = next((t for t in tests if f.stem.startswith(t + "_")), f.stem.split("_")[0])
        out.setdefault(test, {})[f.stem[len(test) + 1:]] = f
    return out


def main() -> None:
    if len(sys.argv) < 2:
        raise SystemExit(__doc__)
    trace_dir = Path(sys.argv[1])
    out_dir = Path(sys.argv[2]) if len(sys.argv) > 2 else trace_dir
    out_dir.mkdir(parents=True, exist_ok=True)
    for test, robots in by_test(trace_dir).items():
        for robot, path in robots.items():
            fig = plot_run(path, f"{test} · {robot}")
            fig.savefig(out_dir / f"{path.stem}.png", dpi=110)
            plt.close(fig)
        fig = plot_overlay(robots, f"{test} · all robots")
        fig.savefig(out_dir / f"{test}_all.png", dpi=110)
        plt.close(fig)
    print(f"plots in {out_dir}")


if __name__ == "__main__":
    main()
