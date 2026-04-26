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
        y = ball_pos.y + ((dy / dist) * offset_dist)
    }
    
    -- Return the calculated point back to the caller
    return point
end

function kick_to_point.is_on_point(robot_id, team, target, tolerance)
    local r_state = get_robot_state(robot_id, team)
    return math.sqrt((r_state.x - target.x)^2 + (r_state.y - target.y)^2) <= tolerance
end

function is_facing_point(robotId, team, target_point, tolerance)
    -- Fetch the robot's current state (adjust this function to match your API)
    local robot_state = get_robot_state(robotId, team) 
    
    -- Calculate the angle from the robot to the target point
    local target_angle = math.atan(target_point.y - robot_state.y, target_point.x - robot_state.x)
    
    -- Get the absolute difference between the robot's current angle and the target angle
    local angle_diff = math.abs(robot_state.orientation - target_angle)
    
    -- Normalize the angle difference to be within -PI and PI
    while angle_diff > math.pi do
        angle_diff = angle_diff - (2 * math.pi)
    end
    angle_diff = math.abs(angle_diff)
    
    -- Return true if the difference is within the allowed tolerance
    return angle_diff <= tolerance
end

function kick_to_point.process(robotId, team, target)
    -- Get the point to move to before kicking
    draw_point(target.x, target.y, true, {r=1.0, g=0.0, b=0.0}) -- Green point for the target
    local point = kick_to_point.get_kick_point(target, 0.09+0.05)


    -- Visualize the target point for debugging
    draw_point(point.x, point.y)
    local ball_pos = get_ball_state()
    
    if kick_to_point.is_on_point(robotId, team, point, 0.1) and is_facing_point(robotId, team, ball_pos, 0.1) then
        return true
    end
    -- Move to the calculated point
    
    face_to(robotId, team, {x = ball_pos.x, y = ball_pos.y})
    move_to(robotId, team, {x = point.x, y = point.y})

    return false
end

return kick_to_point