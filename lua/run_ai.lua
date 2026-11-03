-- Root entry point: run the AI framework experiment.
-- The engine loads this file and calls `process()` once per tick.
--
-- Every blue robot on the field runs the AI (attacking the goal at x = +4.5); roles are
-- assigned each tick (AI/calculator/role_assigner): attacker, one supporter, a goalkeeper
-- with 3+ robots, defenders for the rest. Add or remove blue robots freely. Yellow defends
-- (tests/lib/defense_2v2, shared with the headless test lua/tests/attack_2v2.lua):
--   yellow 0 marks the blue robot farther from the ball (cuts passes to it),
--   yellow 1 keeps the goal.
-- Nothing is teleported: place the robots and the ball where you want (GUI or console),
-- then reload this file; they start from where they are.

local ai = require("AI.main")
local defense = require("tests.lib.defense_2v2")

function process()
    ai.process()
    defense.process()
end
