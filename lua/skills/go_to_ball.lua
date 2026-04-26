local go_to_ball = {}

local utils = require("utils.utils")
local has_the_ball = utils.has_the_ball

--- Calculates a point on the line between the robot and the ball
local function get_approach_point(robot_pos, ball_pos, offset_dist)
    local dx = robot_pos.x - ball_pos.x
    local dy = robot_pos.y - ball_pos.y
    local dist = math.sqrt(dx^2 + dy^2)
    
    if dist < 0.001 then dist = 0.001 end
    
    local point = {
        x = ball_pos.x + ((dx / dist) * offset_dist),
        y = ball_pos.y + ((dy / dist) * offset_dist)
    }
    
    return point
end

local function custom_atan2(y, x)
    if x > 0 then return math.atan(y / x)
    elseif x < 0 and y >= 0 then return math.atan(y / x) + math.pi
    elseif x < 0 and y < 0 then return math.atan(y / x) - math.pi
    elseif x == 0 and y > 0 then return math.pi / 2
    elseif x == 0 and y < 0 then return -(math.pi / 2)
    else return 0 end
end

local function is_on_point(robot_pos, target, tolerance)
    return math.sqrt((robot_pos.x - target.x)^2 + (robot_pos.y - target.y)^2) <= tolerance
end

local function is_facing_point(robot_pos, target_point, tolerance)
    local dx = target_point.x - robot_pos.x
    local dy = target_point.y - robot_pos.y
    
    local target_angle = custom_atan2(dy, dx)
    
    local angle_diff = math.abs(robot_pos.orientation - target_angle)
    
    while angle_diff > math.pi do
        angle_diff = angle_diff - (2 * math.pi)
    end
    angle_diff = math.abs(angle_diff)
    
    return angle_diff <= tolerance
end

function go_to_ball.process(robotId, team)
    local ball_pos = get_ball_state()
    local robot_pos = get_robot_state(robotId, team)
    
    -- Configuration thresholds
    local APPROACH_OFFSET = 0.14 -- Safe distance to approach first
    local TOUCH_OFFSET = 0.05    -- Closer distance to drive directly into the ball
    local DIST_TOLERANCE = 0.05
    local ANGLE_TOLERANCE = 0.1

    -- Get the initial staging point
    local approach_point = get_approach_point(robot_pos, ball_pos, APPROACH_OFFSET)

    draw_point(approach_point.x, approach_point.y)

    -- Phase 2: If we are on the approach point and facing the ball, go for the touch!
    if is_on_point(robot_pos, approach_point, DIST_TOLERANCE) and is_facing_point(robot_pos, ball_pos, ANGLE_TOLERANCE) then
        
        -- Calculate the closer point to push into the ball
        local touch_point = get_approach_point(robot_pos, ball_pos, TOUCH_OFFSET)
        
        -- Move directly (bypassing obstacle avoidance to grab the ball)
        move_direct(robotId, team, {x = touch_point.x, y = touch_point.y})
        
        -- Turn on the dribbler 
        -- NOTE: Replace `activate_dribbler` with whatever function your API uses!
        dribbler(robotId, team, 5) 
        
        -- The skill is only "done" when the ball is securely in the dribbler
        if has_the_ball(robotId, team) then
            return true
        end
        
        return false
    end

    -- Phase 1: Move to the calculated approach point and keep facing the ball
    face_to(robotId, team, {x = ball_pos.x, y = ball_pos.y})
    move_to(robotId, team, {x = approach_point.x, y = approach_point.y})

    return false
end

return go_to_ball