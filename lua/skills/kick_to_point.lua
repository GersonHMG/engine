local kick_to_point = {}


function kick_to_point.get_kick_point(kick_target, offset_dist)
    -- Get current ball position
    local ball_pos = get_ball_state()

    -- Calculate direction from target to ball
    local dx = ball_pos.x - kick_target.x
    local dy = ball_pos.y - kick_target.y
    local dist = math.sqrt(dx^2 + dy^2)
    
    -- Prevent division by zero
    if dist == 0 then dist = 0.001 end
    
    -- Calculate the final approach point
    local point = {
        x = ball_pos.x + ((dx / dist) * offset_dist),
        y = ball_pos.y + ((dy / dist) * offset_dist),
        -- Calculate angle to face the target (using math.atan2 for safety)
        theta = math.atan2(kick_target.y - ball_pos.y, kick_target.x - ball_pos.x)
    }
    
    -- Return the calculated point back to the caller
    return point
end

function kick_to_point.process(robotId, team, target)
    -- Get the point to move to before kicking
    local point = kick_to_point.get_kick_point(target, 0.3)
    -- Visualize the target point for debugging
    draw_point(point.x, point.y) 


    return false
end

return kick_to_point