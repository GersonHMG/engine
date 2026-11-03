#!/bin/bash
# Runs lua/tests/attack.lua on every mode and start set, in parallel, into one run folder.
#
#   lua/tests/run_attack.sh <session folder> <NN-slug>      (from the repository root)
#   e.g. lua/tests/run_attack.sh sessions/strategy-2026-11-04 03-advance-kick
#
# Writes <session>/runs/<NN-slug>/<mode>-<set>.txt (one line per episode + SUMMARY),
# .json (trajectories of the first 12 episodes, for the replay page) and .err (engine log),
# then prints the six SUMMARY lines. Uses the release binary (cargo build --release).
set -u
SESSION=${1:?session folder}
R=$SESSION/runs/${2:?run name}
ENGINE=${ENGINE:-./target/release/engine}
mkdir -p "$R"
for mode in 2v2 3v3; do
  for set in tuning holdout named; do
    ( MODE=$mode SEEDS=$set RECORD=$R/$mode-$set.json \
        "$ENGINE" --headless --max-time 6000 lua/tests/attack.lua \
        > "$R/$mode-$set.txt" 2> "$R/$mode-$set.err" ) &
  done
done
wait
for mode in 2v2 3v3; do
  for set in tuning holdout named; do
    tail -n 1 "$R/$mode-$set.txt" | grep SUMMARY || { echo "FAILED $mode-$set"; tail -n 3 "$R/$mode-$set.err"; }
  done
done
