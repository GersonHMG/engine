-- Move the ball to a desired target

local move_ball = {}
local go_to_ball = require("skills.go_to_ball")
local utils = require("utils.utils")
local has_the_ball = utils.has_the_ball


function move_ball.process(robotId, team, target)
    -- Move to ball
    local ball_pos = get_ball_state()
    local tolerance = 0.2
    
    local has_the_ball = has_the_ball(robotId, team)
    -- If we have the ball, move towards the target
    if has_the_ball and math.sqrt((ball_pos.x - target.x)^2 + (ball_pos.y - target.y)^2) > tolerance then
        move_direct(robotId, team, target)
        dribbler(robotId, team, 5)
        return false
    elseif not has_the_ball then
        -- Move to the ball if we don't have it
        go_to_ball.process(robotId, team)
        return false
    end

    return true
end


return move_ball