local robotId = 0
local team = 0
local move_ball = require("skills.move_ball")

function process()
   move_ball.process(robotId, team, {x = 0.0, y = 0.0})
end