local robotId = 0
local targetRobotId = 1
local team = 0

local PlayEngine = require("play_engine.play_engine")

grsim.teleport_robot(targetRobotId, team, 1.0, 1.0, 0.0)
grsim.teleport_robot(robotId, team, -0.5, -0.5, 0.0)

grsim.teleport_ball(0.0, 0.0)


-- Instantiate a new play object
local play_engine = PlayEngine.new("lua/plays/pass.play")

-- Your main engine loop
function process()
    play_engine:process()
end