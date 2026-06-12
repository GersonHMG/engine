local team = 0

-- Import challenge module
local Challenge = require("challenge")

-- Initial setup: teleport robots to starting positions and ball to center
grsim.teleport_robot(0, team, -0.8, 0.8, 0.0)
grsim.teleport_robot(1, team, -0.8, -0.8, 0.0)
grsim.teleport_robot(2, team, 0.8, 0.0, 0.0)

grsim.teleport_ball(0.0, 0.0)

-- Your main engine loop
function process()
    Challenge.process()
end