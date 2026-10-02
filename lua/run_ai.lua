-- Root entry point: run the AI framework experiment.
-- The engine loads this file and calls `process()` once per tick.

local ai = require("AI.main")

-- 1. Setup the field physically (runs once, when this file is loaded).
--    Scenario: two blue robots coordinate (one attacker, one supporter)
--    against frozen yellow robots. Yellow robots are only placed here and
--    never receive commands, so they stay where they are.
grsim.teleport_robot(0, 0, -0.5, 0.0, 0.0)  -- Blue 0: starts next to the ball
grsim.teleport_robot(1, 0, -1.0, 1.5, 0.0)  -- Blue 1: starts behind, to the side
grsim.teleport_robot(0, 1, 1.5, 0.0, 0.0)   -- Yellow 0: frozen, blocks the straight line
grsim.teleport_robot(1, 1, 3.0, -1.0, 0.0)  -- Yellow 1: frozen, near the goal
grsim.teleport_ball(0.0, 0.0)               -- ball at the field center

-- 2. Run one tick of the experiment.
function process()
	ai.process()
end
