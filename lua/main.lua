local robotId = 0
local targetRobotId = 1
local team = 0

local loop = require("play.pases")


grsim.teleport_robot(targetRobotId, team, 1.0, 1.0, 0.0)
grsim.teleport_robot(robotId, team, -0.5, -0.5, 0.0)

grsim.teleport_ball(0.0, 0.0)

function process()
   loop.process(team)
end