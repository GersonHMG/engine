local m = {}
local TwoAttackers = require("AI.team_behavior.two_attackers")

-- 1. Setup the field physically (Do this only once!)
grsim.teleport_robot(0, 0, -0.5, -0.5, -0.0) -- Blue 0
grsim.teleport_robot(1, 0, -0.5, 1.0, -9.0)  -- Blue 1
grsim.teleport_robot(0, 1, 0.5, 0.5, -0.0)   -- Yellow 0 (example coordinates)
grsim.teleport_robot(1, 1, 0.5, -1.0, 9.0)   -- Yellow 1 (example coordinates)
grsim.teleport_ball(-0.35, -0.5)

-- 2. Mount both teams
-- (Team 0, Robot 0, Robot 1)
local blue_team = TwoAttackers.new(0, 0, 1)


function m.process()
    -- 3. Run both AI brains simultaneously
    blue_team:process()
end

return m