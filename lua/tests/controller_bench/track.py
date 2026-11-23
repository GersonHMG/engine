# /// script
# requires-python = ">=3.10"
# dependencies = ["mlflow>=3,<4", "matplotlib>=3.8"]
# ///
"""Runs the controller benchmark and logs it to MLflow.

    uv run lua/tests/controller_bench/track.py --note "baseline"
    uv run lua/tests/controller_bench/track.py --controller my_ctrl --note "latency comp 66 ms"
    uv run lua/tests/controller_bench/track.py --set CB_ANGLE_NOISE=0 --conditions realistic

Then look at the runs:

    uvx --python 3.12 --from "mlflow>=3,<4" mlflow ui --backend-store-uri sqlite:///lua/tests/controller_bench/mlflow/mlflow.db

One invocation is one parent run (the experiment as you named it with --note). Under it,
one child run per condition × robot, with every metric of every test as
`<test>.<metric>` (e.g. `go_to_point.reach_s`) and `<test>.ok` = 1 when the test's
primary metric exists (reached / finished / kicked). The parent has the primary metrics
of all children as `<condition>.<robot>.<test>.<metric>`.

Logged with each run: git commit and whether the tree had changes (then the diff is
attached), the controller's source and parameters, the simulator noise/delay, the robot
profiles, the engine's output and the true trajectories (CB_TRACE).

Run from the repository root. The engine must be built (cargo build); --build builds it.
"""

from __future__ import annotations

import argparse
import csv
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

os.environ.setdefault("MLFLOW_DISABLE_AGENT_HINT", "1")

import mlflow  # noqa: E402
from mlflow.tracking import MlflowClient  # noqa: E402

import plot  # noqa: E402  (plot.py, next to this script)

ROOT = Path(__file__).resolve().parents[3]
BENCH = ROOT / "lua" / "tests" / "controller_bench"
EXPERIMENT = "controller_bench"


def git(*args: str) -> str:
    out = subprocess.run(["git", *args], cwd=ROOT, capture_output=True, text=True, encoding="utf-8")
    return out.stdout.strip()


def main_checkout() -> Path:
    """The repository's main checkout, also when this runs in a git worktree."""
    common = git("rev-parse", "--path-format=absolute", "--git-common-dir")
    return Path(common).parent if common else ROOT


MAIN = main_checkout()
# One store for every checkout: runs from worktree sessions land next to the others.
STORE = Path(os.environ.get("CB_MLFLOW_STORE", MAIN / "lua" / "tests" / "controller_bench" / "mlflow"))
# This checkout's engine, else the main checkout's (a worktree has no target/ until it builds).
ENGINE = next((p for p in (ROOT / "target" / "debug" / "engine.exe", MAIN / "target" / "debug" / "engine.exe")
               if p.exists()), ROOT / "target" / "debug" / "engine.exe")


def metric_key(text: str) -> str:
    """A metric/param name MLflow accepts (letters, digits, _ - . / and spaces)."""
    return re.sub(r"[^\w\-./ ]", "_", text)


def read_meta(path: Path) -> dict[str, str]:
    meta = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        if "=" in line:
            k, v = line.split("=", 1)
            meta[k] = v
    return meta


def read_results(path: Path) -> dict[tuple[str, str], dict[str, float | None]]:
    """{(robot, test): {metric: value or None}} from the benchmark's CSV."""
    out: dict[tuple[str, str], dict[str, float | None]] = {}
    with path.open(encoding="utf-8") as f:
        for row in csv.DictReader(f):
            value = float(row["value"]) if row["value"] else None
            out.setdefault((row["robot"], row["test"]), {})[row["metric"]] = value
    return out


def run_engine(env: dict[str, str]) -> str:
    cmd = [str(ENGINE), "--headless", "--max-time", "600", "lua/tests/controller_bench/run.lua"]
    proc = subprocess.run(cmd, cwd=ROOT, env=env, capture_output=True)
    output = (proc.stdout + proc.stderr).decode("utf-8", errors="replace")
    if proc.returncode != 0 or "SUMMARY" not in output:
        sys.stderr.write(output[-3000:])
        raise SystemExit(f"engine run failed (exit {proc.returncode})")
    return output


def experiment_id(client: MlflowClient) -> str:
    exp = client.get_experiment_by_name(EXPERIMENT)
    if exp:
        return exp.experiment_id
    return client.create_experiment(EXPERIMENT, artifact_location=(STORE / "artifacts").as_uri())


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--controller", default="bang_bang", help="controllers/<name>.lua")
    ap.add_argument("--conditions", default="ideal,realistic", help="comma list: ideal, realistic")
    ap.add_argument("--robots", default=None, help="comma list of robots/<name>.toml (default: the benchmark's)")
    ap.add_argument("--tests", default=None, help="comma list of tests (default: all)")
    ap.add_argument("--set", action="append", default=[], metavar="CB_X=V",
                    help="benchmark override, e.g. CB_ANGLE_NOISE=0, or SIM_SEED=1 for another noise sequence (repeatable)")
    ap.add_argument("--note", default="", help="what this experiment tries; the run's name")
    ap.add_argument("--build", action="store_true", help="cargo build before running")
    args = ap.parse_args()

    if args.build:
        subprocess.run(["cargo", "build"], cwd=ROOT, check=True)
    if not ENGINE.exists():
        raise SystemExit(f"{ENGINE} not found: cargo build first, or pass --build")
    controller_file = BENCH / "controllers" / f"{args.controller}.lua"
    if not controller_file.exists():
        raise SystemExit(f"{controller_file} not found")

    overrides = {}
    for item in args.set:
        key, _, value = item.partition("=")
        if not (key.startswith("CB_") or key == "SIM_SEED") or not value:
            raise SystemExit(f"--set expects CB_NAME=value or SIM_SEED=n, got {item!r}")
        overrides[key] = value

    commit = git("rev-parse", "HEAD")
    dirty = bool(git("status", "--porcelain", "--untracked-files=no"))

    STORE.mkdir(parents=True, exist_ok=True)
    mlflow.set_tracking_uri(f"sqlite:///{(STORE / 'mlflow.db').as_posix()}")
    client = MlflowClient()
    exp_id = experiment_id(client)

    conditions = [c.strip() for c in args.conditions.split(",") if c.strip()]
    run_name = args.note or f"{args.controller}"
    with tempfile.TemporaryDirectory() as tmp_str, \
            mlflow.start_run(experiment_id=exp_id, run_name=run_name) as parent:
        tmp = Path(tmp_str)
        mlflow.set_tags({
            "git.commit": commit, "git.dirty": str(dirty), "git.branch": git("rev-parse", "--abbrev-ref", "HEAD"),
            "controller": args.controller, "note": args.note, "mlflow.note.content": args.note,
        })
        mlflow.log_params({"controller": args.controller, "conditions": ",".join(conditions),
                           **{f"override.{k}": v for k, v in overrides.items()}})
        mlflow.log_artifact(str(controller_file), "controller")
        for profile in sorted((ROOT / "robots").glob("*.toml")):
            mlflow.log_artifact(str(profile), "robots")
        if dirty:
            diff = tmp / "uncommitted.diff"
            diff.write_text(git("diff", "HEAD"), encoding="utf-8")
            mlflow.log_artifact(str(diff), "git")

        summary = []
        scores: dict[str, float] = {}
        for condition in conditions:
            trace = tmp / f"trace_{condition}"
            trace.mkdir()
            env = dict(os.environ, CB_CONTROLLER=args.controller, CB_CONDITION=condition, CB_TRACE=str(trace),
                       **overrides)
            if args.robots:
                env["CB_ROBOTS"] = args.robots
            if args.tests:
                env["CB_TESTS"] = args.tests
            print(f"running {args.controller} / {condition} ...", flush=True)
            output = run_engine(env)

            meta_path = BENCH / "results" / "latest.meta"
            meta = read_meta(meta_path)
            csv_path = ROOT / meta["csv"]
            results = read_results(csv_path)
            log = tmp / f"engine_{condition}.txt"
            log.write_text(output, encoding="utf-8")
            mlflow.log_artifact(str(log), "logs")
            mlflow.log_artifact(str(csv_path), "results")

            label = meta["condition"]  # with any overrides in it
            primary = {k.split(".", 1)[1]: v for k, v in meta.items() if k.startswith("primary.")}
            sim_params = {k: v for k, v in meta.items() if k.startswith(("sim.", "controller."))}
            if condition == conditions[0]:
                mlflow.log_params({k: v for k, v in sim_params.items() if k.startswith("controller.")})

            condition_score = 0.0
            for robot in meta["robots"].split(","):
                with mlflow.start_run(experiment_id=exp_id, run_name=f"{label} · {robot}", nested=True):
                    mlflow.set_tags({"git.commit": commit, "git.dirty": str(dirty), "controller": args.controller,
                                     "note": args.note})
                    mlflow.log_params({"controller": args.controller, "condition": label, "robot": robot,
                                       **sim_params, **{f"override.{k}": v for k, v in overrides.items()}})
                    metrics = {}
                    for test in meta["tests"].split(","):
                        values = results.get((robot, test), {})
                        for name, value in values.items():
                            if value is not None:
                                metrics[f"{test}.{name}"] = value
                        p = values.get(primary.get(test, ""))
                        metrics[f"{test}.ok"] = 0.0 if p is None else 1.0
                        if p is not None:
                            mlflow.log_metric(metric_key(f"{label}.{robot}.{test}.{primary[test]}"), p, run_id=parent.info.run_id)
                        summary.append((label, robot, test, primary.get(test, "?"), p))
                    # score: the sum of this robot's test costs (run.lua's SCORE, per robot).
                    metrics["score"] = sum(v for k, v in metrics.items() if k.endswith(".cost"))
                    condition_score += metrics["score"]
                    mlflow.log_metrics(metrics)
                    for f in sorted(trace.glob(f"*_{robot}.csv")):
                        mlflow.log_artifact(str(f), "trajectories")
                        fig = plot.plot_run(f, f"{f.stem.removesuffix('_' + robot)} · {robot} · {label} · {args.controller}")
                        mlflow.log_figure(fig, f"plots/{f.stem}.png")
                        plot.plt.close(fig)

            mlflow.log_metric(metric_key(f"{label}.score"), condition_score)
            scores[label] = condition_score

            # All robots of each test on one field, in the parent run.
            for test, robots in plot.by_test(trace).items():
                fig = plot.plot_overlay(robots, f"{test} · {label} · {args.controller}")
                mlflow.log_figure(fig, f"plots/{metric_key(label)}/{test}_all.png")
                plot.plt.close(fig)

        # score.total: the objective to minimize (sum over the conditions that ran).
        mlflow.log_metric("score.total", sum(scores.values()))

        print(f"\nlogged run '{run_name}' ({parent.info.run_id}) to experiment '{EXPERIMENT}'"
              f"{' (uncommitted changes: diff attached)' if dirty else ''}")
        for label, robot, test, metric, value in summary:
            shown = "NO" if value is None else f"{value:.3g}"
            print(f"  {label:<12} {robot:<14} {test:<14} {metric:<9} {shown}")
        print("\nview: uvx --python 3.12 --from \"mlflow>=3,<4\" mlflow ui --backend-store-uri "
              f"sqlite:///{(STORE / 'mlflow.db').as_posix()}")
        for label, value in scores.items():
            print(f"SCORE {label} {value:.2f}")
        print(f"SCORE total {sum(scores.values()):.2f}  (run {parent.info.run_id})")


if __name__ == "__main__":
    main()
