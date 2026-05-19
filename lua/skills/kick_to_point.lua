local kick_to_point = {}

local utils = require("utils.utils")
local has_the_ball = utils.has_the_ball

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



function kick_to_point.process(robotId, team, target)
    -- Get the point to move to before kicking
    draw_point(target.x, target.y, true, {r=1.0, g=0.0, b=0.0}) -- Green point for the target
    local ball_pos = get_ball_state()
    
    local point = get_kick_point(target, 0.09+0.20)

    -- Visualize the target point for debugging
    draw_point(point.x, point.y)
    local robot_pos = get_robot_state(robotId, team)
    if is_on_kicking_line(robot_pos, ball_pos, point, 0.05) and is_facing_point(robotId, team, ball_pos, 0.1) then
        local point = get_kick_point(target, 0.045)
        move_direct(robotId, team, {x = point.x, y = point.y})
        
        if has_the_ball(robotId, team) then
            kickx(robotId, team)
            return true
        end
        return false
    end

    -- Move to the calculated point
    face_to(robotId, team, {x = ball_pos.x, y = ball_pos.y})
    move_to(robotId, team, {x = point.x, y = point.y})

    return false
end


return kick_to_point