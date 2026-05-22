local robotId = 0
local targetRobotId = 1
local team = 0

local loop = require("play_engine.pases_demo")
local motor = require("utils.table_to_tactic")

grsim.teleport_robot(targetRobotId, team, 1.0, 1.0, 0.0)
grsim.teleport_robot(robotId, team, -0.5, -0.5, 0.0)

grsim.teleport_ball(0.0, 0.0)

function process()
   motor.cargar_jugada("lua/utils/plays/pass.play")
   motor.update(team)
   loop.process(team)
end