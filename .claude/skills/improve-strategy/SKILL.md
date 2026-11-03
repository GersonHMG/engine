---
name: improve-strategy
description: Improve the blue team's game strategy (lua/AI actions, roles, scoring) until it beats the fixed 2v2 and 3v3 test defenses, by measuring episodes in the headless simulator. Use when asked to improve the attack, the AI framework's decisions, or the win rate against the defense.
---

Follow `docs/improve-strategy-loop.md` exactly. It holds the goal, the rules, the harness,
the loop and the pitfalls already found. Read it, `simulator/CLAUDE.md` and
`docs/action-scoring.md` before changing anything. Run every test yourself; do not ask the
user to run them.

Non-negotiable:
- Only the goalkeeper may enter its own goalkeeper area, and never the opponent's. Keep
  `utils/area_guard.lua` working, and keep the `foul` count at 0.
- Do not change the Rust engine, the yellow defenses (`lua/tests/lib/defense_*.lua`), or
  the framework backbone (`Role:choose`, the `Action` interface, `role_assigner`,
  `ball_owner`, the tick loop in `AI/main.lua`).
- One change per measured run: all six sets (`lua/tests/run_attack.sh`). Keep it only by
  the keep rule in section 3 of the doc, write hypothesis.md and verdict.md for every
  run, and commit kept changes with their numbers.
- Stop and report when the stop rule in section 3 says so.
