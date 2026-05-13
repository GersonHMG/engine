local robotId = 0
local targetRobotId = 1  -- Arreglado para que todo calce perfecto
local team = 0

local move_to = require("skills.move")
local move_ball = require("skills.move_ball")
local kick = require("skills.kick_to_point")
local pass = require("tactics.active.pass")
local recivepass = require("tactics.active.receive_pass")
local position_for_pass = require("tactics.non_active.position_for_pass")
local pase = require("tactics.pase")

local state = 0

grsim.teleport_robot(targetRobotId, team, 1.0, 1.0, 0.0)
grsim.teleport_robot(robotId, team, -0.5, -0.5, 0.0)

grsim.teleport_ball(0.0, 0.0)

function process()
   pase.process(robotId, team, targetRobotId)
end