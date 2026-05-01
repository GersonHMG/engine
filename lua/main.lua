local robotId = 0
local team = 0
local move_ball = require("skills.move_ball")

function process()
   ---move_ball.process(robotId, team, {x = 0.0, y = 0.0})
   --local path = plan_path({x = 0.0, y = 0.0}, {x = 2.0, y = 1.0})
   local path = move_to_path(robotId, team, {x = 0.0, y = 0.0})
   draw_line(path)
end