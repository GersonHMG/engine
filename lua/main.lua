local robotId = 0
local targetRobotId = 1
local team = 0

local kick_to_point = require("skills.kick_to_point")

-- Your main engine loop
function process()
    kick_to_point.process(0, 0, {x=0.0, y=0.0})
end