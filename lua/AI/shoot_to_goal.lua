

local m = {}
local shoot = require("tactics.active.shoot")

-- Set the initial configuration
local team = 0 -- 0 for blue, 1 for yellow
local robotId = 0


grsim.teleport_robot(robotId, team, 2.8, 0.05, 0.0)

grsim.teleport_ball(3.03, 0.05)

function m.process()
    shoot.process(robotId, team)

end

return m