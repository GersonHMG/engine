"""Recordings made with lua/tests/collect_dynamics.lua (grSim, real robots, or the built-in simulator).

    python -m nn_sim.grsim_data check logs/dyn_mix_*.csv     # quality report, run after each recording

CSV columns (one row per engine tick):
    wall_t   wall-clock time of the tick, s (engine `wall_time()`)
    tick     engine tick counter of the script
    episode, seg, phase   episode number, signal type (data.SEGMENTS), 0 = settling after teleport, 1 = excitation
    active   1 when the robot was in vision this tick
    vt_cmd, vn_cmd, w_cmd command sent this tick (robot frame)
    x, y, theta          pose from vision (raw)
    vx, vy, omega        velocity from the engine's tracker (Kalman filter)

Row k and row k+1 (consecutive ticks of the same episode, both phase 1 and active) form one
transition (state_k, command_k) -> state_{k+1}, as in the simulator datasets.
"""

import argparse
import json
from pathlib import Path

import numpy as np
import pandas as pd

from . import data, reference

TICK = 1.0 / 60.0


def load_csv(path):
    path = Path(path)
    df = pd.read_csv(path)
    meta_path = path.with_suffix(".json")
    meta = json.loads(meta_path.read_text()) if meta_path.exists() else {}
    return df, meta


def transition_mask(df):
    """Rows k that start a valid transition k -> k+1."""
    nxt = df.shift(-1)
    return (
        (df["phase"] == 1) & (nxt["phase"] == 1) & (df["active"] == 1) & (nxt["active"] == 1)
        & (nxt["episode"] == df["episode"]) & (nxt["tick"] == df["tick"] + 1)
    ).to_numpy()


def diff_velocities(df, clock="wall"):
    """World velocity and ω from vision poses by central differences (per episode), over wall
    time or, for headless runs (faster than real time), over ticks."""
    out = np.full((len(df), 3), np.nan)
    for _, g in df.groupby("episode"):
        idx = g.index.to_numpy()
        if len(idx) < 3:
            continue
        t = g["wall_t"].to_numpy() if clock == "wall" else g["tick"].to_numpy() * TICK
        th = np.unwrap(g["theta"].to_numpy())
        for j, col in enumerate([g["x"].to_numpy(), g["y"].to_numpy(), th]):
            out[idx, j] = np.gradient(col, t)
    return out


def body(vx, vy, theta):
    c, s = np.cos(theta), np.sin(theta)
    return c * vx + s * vy, -s * vx + c * vy


def check(path):
    df, meta = load_csv(path)
    src = meta.get("source", "?")
    print(f"\n=== {Path(path).name}  (mode {meta.get('mode', '?')}, seed {meta.get('seed', '?')}, source {src})")
    exc = df[df["phase"] == 1]
    headless = "headless" in src
    dt = np.diff(df["wall_t"].to_numpy())
    same_ep = (df["episode"].diff() == 0).to_numpy()[1:]
    gaps = np.diff(df["tick"].to_numpy())[same_ep]
    mask = transition_mask(df)
    warn = []

    print(f"rows {len(df)}, episodes {df['episode'].nunique()}, excitation {len(exc) * TICK / 60:.1f} min "
          f"(of {len(df) * TICK / 60:.1f} min logged), usable transitions {mask.sum()}")
    if headless:
        print("timing: headless run (faster than real time), wall-clock checks skipped")
    else:
        late = (dt > 1.5 * TICK).mean()
        print(f"tick period: median {np.median(dt) * 1e3:.2f} ms, p1 {np.percentile(dt, 1) * 1e3:.2f}, p99 {np.percentile(dt, 99) * 1e3:.2f}, "
              f"max {dt.max() * 1e3:.1f} ms; late ticks (> 25 ms) {late:.2%}")
        if late > 0.02:
            warn.append(f"{late:.1%} of ticks are late (> 25 ms): close other programs, keep the engine window visible")
    if (gaps != 1).any():
        warn.append(f"{(gaps != 1).sum()} jumps in the tick counter inside episodes")

    inactive = 1 - exc["active"].mean() if len(exc) else 1
    print(f"vision: robot missing in {inactive:.2%} of excitation ticks")
    if inactive > 0.01:
        warn.append(f"robot not in vision in {inactive:.1%} of ticks")
    moving = (exc["vt_cmd"].abs() + exc["vn_cmd"].abs() + exc["w_cmd"].abs()) > 0.2
    same = (exc[["x", "y", "theta"]].diff().abs().sum(1) == 0) & moving & moving.shift(1, fill_value=False)
    print(f"vision: repeated pose while commanded to move in {same.mean():.2%} of excitation ticks (duplicate/missed frames)")
    if same.mean() > 0.05:
        warn.append(f"{same.mean():.1%} repeated vision frames: vision is slower than 60 Hz or frames are dropped")

    # Command coverage and measured response
    cmd_speed = np.hypot(exc["vt_cmd"], exc["vn_cmd"])
    vel = diff_velocities(df, "tick" if headless else "wall")
    ub, vb = body(vel[:, 0], vel[:, 1], df["theta"].to_numpy())
    sel = (df["phase"] == 1).to_numpy() & (df["active"] == 1).to_numpy()
    print(f"commands: |v| up to {cmd_speed.max():.2f} m/s (p50 {cmd_speed.median():.2f}), |w| up to {exc['w_cmd'].abs().max():.2f} rad/s")
    print(f"measured (vision differences): |v| up to {np.nanpercentile(np.hypot(ub, vb)[sel], 99.5):.2f} m/s (p99.5), "
          f"|w| up to {np.nanpercentile(np.abs(vel[sel, 2]), 99.5):.2f} rad/s (p99.5)")

    # Delay: replay each episode's commands through the built-in simulator's physics, delayed by
    # L ticks, and keep the L whose velocities best match the measured ones. Acceleration limits
    # are part of the model, so only the extra (transport) delay is left in L.
    lags = np.arange(0, 13)
    best = []
    for _, g in df[sel].groupby("episode"):
        if len(g) < 120 or len(best) >= 25 or (np.diff(g["tick"].to_numpy()) != 1).any():
            continue
        u = g[["vt_cmd", "vn_cmd", "w_cmd"]].to_numpy()
        meas = vel[g.index.to_numpy()]
        st = np.tile(np.r_[g[["x", "y", "theta"]].to_numpy()[0], meas[0]], (len(lags), 1))
        err = np.zeros(len(lags))
        for k in range(len(u) - 1):
            cmd = np.array([u[k - L] if k >= L else np.zeros(3) for L in lags])
            st = reference.step(st, cmd)
            if k >= lags[-1] and np.isfinite(meas[k + 1]).all():
                err += ((st[:, 3:6] - meas[k + 1]) ** 2).sum(1)
        best.append(int(lags[np.argmin(err)]))
    if best:
        lag = np.median(best)
        print(f"response delay beyond the simulator model: ~{lag:.0f} ticks ({lag * TICK * 1e3:.0f} ms), "
              f"median over {len(best)} episodes (spread {min(best)}-{max(best)})")

    # How far is this from the built-in simulator's physics? (one-step, positions)
    if mask.sum():
        s = df[["x", "y", "theta", "vx", "vy", "omega"]].to_numpy()
        u = df[["vt_cmd", "vn_cmd", "w_cmd"]].to_numpy()
        pred = reference.step(s[mask], u[mask])
        true = s[np.nonzero(mask)[0] + 1]
        dpos = np.hypot(*(pred[:, :2] - true[:, :2]).T)
        dth = np.abs(data.wrap(pred[:, 2] - true[:, 2]))
        print(f"vs built-in simulator physics (one tick ahead, using logged velocities): position error median {np.median(dpos) * 1e3:.3f} mm, "
              f"p99 {np.percentile(dpos, 99) * 1e3:.3f} mm; heading median {np.degrees(np.median(dth)):.4f} deg")

    print("RESULT:", "OK" if not warn else "CHECK -> " + "; ".join(warn))
    return not warn


def main():
    p = argparse.ArgumentParser()
    sub = p.add_subparsers(dest="cmd", required=True)
    c = sub.add_parser("check")
    c.add_argument("files", nargs="+")
    args = p.parse_args()
    ok = all([check(f) for f in args.files])
    raise SystemExit(0 if ok else 1)


if __name__ == "__main__":
    main()
