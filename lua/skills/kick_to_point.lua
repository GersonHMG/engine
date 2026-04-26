local kick_to_point = {}


local function get_kick_point(kick_target, offset_dist)
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

local function is_on_point(robot_id, team, target, tolerance)
    local r_state = get_robot_state(robot_id, team)
    return math.sqrt((r_state.x - target.x)^2 + (r_state.y - target.y)^2) <= tolerance
end

local function is_facing_point(robotId, team, target_point, tolerance)
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

local function is_on_kicking_line(robot_pos, ball_pos, target_pos, tolerance)
    -- Vector from Ball to Target (the direction we want to kick)
    local dx_bt = target_pos.x - ball_pos.x
    local dy_bt = target_pos.y - ball_pos.y
    
    local length_bt = math.sqrt(dx_bt^2 + dy_bt^2)
    if length_bt == 0 then return false end -- Prevent division by zero
    
    -- Normalize the direction vector
    local dir_x = dx_bt / length_bt
    local dir_y = dy_bt / length_bt
    
    -- Vector from Ball to Robot
    local dx_br = robot_pos.x - ball_pos.x
    local dy_br = robot_pos.y - ball_pos.y
    
    -- 1. Check if the robot is BEHIND the ball
    -- We use the dot product. If positive, the robot is in front of the ball.
    local dot_product = dir_x * dx_br + dir_y * dy_br
    
    if dot_product < 0 then
        return false 
    end
    -- 2. Check the perpendicular distance to the line
    -- Using the 2D cross product magnitude
    local distance_to_line = math.abs(dir_x * dy_br - dir_y * dx_br)
    
    return distance_to_line <= tolerance
end


--- Checks if a specific robot currently possesses the ball.
--- @param robotId number
--- @param team number
--- @return boolean
local function has_the_ball(robotId, team)
    -- Fetch current states (adjust function names to match your API)
    local robot_state = get_robot_state(robotId, team)
    local ball_state = get_ball_state()

    -- ---------------------------------------------------------
    -- Configuration Thresholds (Tweak these to fit your robots!)
    -- ---------------------------------------------------------
    -- The maximum distance to be considered "touching" the dribbler. 
    -- (Roughly the robot's radius + ball's radius + small tolerance)
    local DISTANCE_THRESHOLD = 0.12 
    
    -- The maximum angular spread of the dribbler mouth (in radians).
    -- 0.35 radians is roughly 20 degrees.
    local ANGLE_THRESHOLD = 0.35 
    -- ---------------------------------------------------------

    -- 1. Calculate the distance between the robot's center and the ball
    local dx = ball_state.x - robot_state.x
    local dy = ball_state.y - robot_state.y
    local distance = math.sqrt(dx^2 + dy^2)

    -- If the ball is too far away, return false immediately
    if distance > DISTANCE_THRESHOLD then
        return false
    end

    -- 2. Calculate the absolute angle from the robot to the ball
    local angle_to_ball = math.atan(dy, dx)

    -- 3. Find the difference between the robot's facing angle and the ball's angle
    local angle_diff = math.abs(robot_state.orientation - angle_to_ball)

    -- Normalize the difference to stay within -PI and PI
    while angle_diff > math.pi do
        angle_diff = angle_diff - (2 * math.pi)
    end
    angle_diff = math.abs(angle_diff)

    -- 4. The robot has the ball if it is close enough AND right in front
    return angle_diff <= ANGLE_THRESHOLD
end




function kick_to_point.process(robotId, team, target)
    -- Get the point to move to before kicking
    draw_point(target.x, target.y, true, {r=1.0, g=0.0, b=0.0}) -- Green point for the target
    local ball_pos = get_ball_state()
    
    local point = get_kick_point(target, 0.09+0.05)

    -- Visualize the target point for debugging
    draw_point(point.x, point.y)
    local robot_pos = get_robot_state(robotId, team)
    if is_on_kicking_line(robot_pos, ball_pos, point, 0.05) and is_facing_point(robotId, team, ball_pos, 0.1) then
        local point = get_kick_point(target, 0.05)
        move_direct(robotId, team, {x = point.x, y = point.y})
        if has_the_ball(robotId, team) then
            kickx(robotId, team)
        end

        return true
    end

    -- Move to the calculated point
    face_to(robotId, team, {x = ball_pos.x, y = ball_pos.y})
    move_to(robotId, team, {x = point.x, y = point.y})

    return false
end

return kick_to_point