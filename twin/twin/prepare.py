"""Clean and smooth every session once, and cache the result (plan §3).

    python -m twin.prepare --data data/synthetic --out data/prepared.pkl
"""

import argparse
import pickle
from concurrent.futures import ProcessPoolExecutor
from functools import partial
from pathlib import Path

import numpy as np

from .data import fit_jerk, load_session, load_split, measurement_noise, prepare


def _prepare(path, noise, jerk):
    return prepare(load_session(path), noise, jerk)


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--data", type=Path, default=Path("data/synthetic"))
    parser.add_argument("--out", type=Path, default=Path("data/prepared.pkl"))
    args = parser.parse_args()

    robots = sorted(p.name for p in args.data.iterdir() if p.is_dir())
    noise = {}
    jerk = {}
    for robot in robots:
        noise[robot] = measurement_noise(load_split(args.data, "noise", [robot]))
        # Smoother tuning by innovation likelihood on two train sessions (no truth used).
        sample = [s for s in load_split(args.data, "train", [robot]) if s.program in ("prbs", "steps")][:2]
        jerk[robot] = fit_jerk(sample, noise[robot])
        print(f"{robot:10s} vision noise std x,y,θ = {np.round(noise[robot], 4)}  jerk q = {jerk[robot]}")

    out = {"noise": noise, "jerk": jerk, "splits": {}}
    with ProcessPoolExecutor() as pool:
        for split in ("train", "val", "test"):
            segments = []
            for robot in robots:
                paths = sorted((args.data / robot / split).glob("*.csv"))
                for segs in pool.map(partial(_prepare, noise=noise[robot], jerk=jerk[robot]), paths):
                    segments.extend(segs)
            out["splits"][split] = segments
            ticks = sum(len(s.cmd) for s in segments)
            print(f"{split:5s}: {len(segments)} segments, {ticks / 3600:.1f} min")
    args.out.parent.mkdir(parents=True, exist_ok=True)
    with open(args.out, "wb") as f:
        pickle.dump(out, f)


if __name__ == "__main__":
    main()
