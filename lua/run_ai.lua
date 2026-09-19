-- Root entry point: run the AI framework experiment.
-- The engine loads this file and calls `process()` once per tick.

local ai = require("AI.main")

-- 1. Setup the field physically (runs once, when this file is loaded).
--    Robot 2 of team 0 is the one AI.main drives; it starts away from the
--    ball so the first decision is go_to_ball and the kick follows.
grsim.teleport_robot(0, 0, -0.5, 0.0, 0.0) -- Blue 2: the experiment robot
grsim.teleport_robot(0, 1, 1.5, 0.0, 0.0) -- Yellow 0: the experiment robot
grsim.teleport_ball(1.0, 0.0)              -- ball at the field center

-- 2. Run one tick of the experiment.
function process()
	ai.process()
end
