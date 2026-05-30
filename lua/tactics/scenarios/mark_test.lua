
local m = {}
local robotId = 0

local team = 0

local targetRobotId = 0
local enemy_team = 1

local mark = require("tactics.non_active.mark")

grsim.teleport_robot(targetRobotId, team, 1.0, 1.0, 0.0)
grsim.teleport_robot(robotId, team, -0.5, -0.5, 0.0)

grsim.teleport_ball(0.0, 0.0)


grsim.teleport_robot(0, 0, -0.5, -0.5, -0.0)
grsim.teleport_robot(1, 0, -0.5, 1.0, -9.0)
grsim.teleport_ball(-0.35, -0.5)
-- Instantiate a new play object

-- Your main engine loop
function m.process()


    mark.process(robotId, team, targetRobotId, enemy_team)
end

return m