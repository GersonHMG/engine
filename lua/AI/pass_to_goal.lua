local m = {}
local pass = require("tactics.active.pass")
local receive = require("tactics.active.receive_pass")


-- Set the initial configuration
local team = 0 -- 0 for blue, 1 for yellow
local robotId = 0
local targetRobotId = 1


grsim.teleport_robot(robotId, team, -0.5, -0.5, -0.0)
grsim.teleport_robot(targetRobotId, team, 1.0, 1.0, -9.0)

grsim.teleport_ball(-0.35, -0.5)

function m.process()
    pass.process(robotId, team, targetRobotId)
    receive.process(targetRobotId, team)

end

return m