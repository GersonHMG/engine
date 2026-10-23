"""Write a session and the twin's free run of it as an engine replay CSV.

The real robot is blue 0 (vision, or the noise-free truth with --truth), the
twin fed the same commands is yellow 0. Open the file in the engine GUI's
replay panel to watch where they part.

    python -m twin.replay data/synthetic/bad_wheel/test/move_to_0.csv --out run.csv
"""

import argparse
from pathlib import Path

import numpy as np

from .data import load_session
from .robots import TICK
from .simulate import Twin

HEADER = "ElapsedTime,RobotID,Team,Vx_Command,Vy_Command,Angular_Command,Pos_X,Pos_Y,Orientation,Vx_Actual,Vy_Actual"


def world_velocity(body, theta):
    c, s = np.cos(theta), np.sin(theta)
    return c * body[:, 0] - s * body[:, 1], s * body[:, 0] + c * body[:, 1]


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("session", type=Path)
    parser.add_argument("--checkpoint", type=Path, default=Path("checkpoints/twin_gru.pt"))
    parser.add_argument("--out", type=Path, default=Path("replay.csv"))
    parser.add_argument("--truth", action="store_true", help="show the noise-free truth (synthetic data) instead of vision")
    args = parser.parse_args()

    session = load_session(args.session)
    twin = Twin.load(args.checkpoint, session.robot)
    if args.truth and session.truth:
        real = session.truth["pose"]
    else:
        # Hold the last detection through dropouts.
        real = session.raw.copy()
        for k in range(1, len(real)):
            if np.isnan(real[k, 0]):
                real[k] = real[k - 1]
        real = np.nan_to_num(real)
    start = session.truth["pose"][0] if session.truth else real[0]
    pred = twin.rollout(tuple(start), session.cmd[:-1])
    twin_pose = np.vstack([start, pred[:, :3]])
    twin_vx, twin_vy = world_velocity(np.vstack([np.zeros(3), pred[:, 3:]]), twin_pose[:, 2])

    with open(args.out, "w") as f:
        f.write(HEADER + "\n")
        for k in range(len(session.cmd)):
            t = k * TICK
            c = session.cmd[k]
            f.write(f"{t:.3f},0,0,{c[0]},{c[1]},{c[2]},{real[k, 0]},{real[k, 1]},{real[k, 2]},0,0\n")
            f.write(f"{t:.3f},0,1,{c[0]},{c[1]},{c[2]},{twin_pose[k, 0]},{twin_pose[k, 1]},{twin_pose[k, 2]},{twin_vx[k]},{twin_vy[k]}\n")
            f.write(f"{t:.3f},-1,-1,0,0,0,0,0,0,0,0\n")
    print(f"wrote {args.out}: blue 0 = real robot, yellow 0 = twin")


if __name__ == "__main__":
    main()
