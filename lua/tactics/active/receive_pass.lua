local receive_pass = {}

function get_distance(p1, p2)
    local dx = p2.x - p1.x
    local dy = p2.y - p1.y
    return math.sqrt(dx^2 + dy^2)
end

--- Executes the receive pass tactic
--- @param robotId number
--- @param team number
function receive_pass.process(robotId, team)
    local ball_pos = get_ball_state()
    local robot_pos = get_robot_state(robotId, team)

    local bx = ball_pos.x
    local by = ball_pos.y
    local rx = robot_pos.x
    local ry = robot_pos.y
    
    -- The distance (in meters or your units) to place the point near the ball.
    -- Adjust this based on how close you want the robot to stand.
    local OFFSET = 0.15
    
    local target_x, target_y
    
    -- 1. Calculate the vector from the BALL pointing towards the ROBOT
    local dx = rx - bx
    local dy = ry - by
    local dist = math.sqrt(dx^2 + dy^2)
    
    -- 2. Check to avoid division by zero if they are occupying the exact same coordinate
    if dist > 0.001 then
        -- Normalize the vector (dx/dist, dy/dist) and multiply by the desired offset
        target_x = bx + (dx / dist) * OFFSET
        target_y = by + (dy / dist) * OFFSET
    else
        target_x = bx
        target_y = by
    end


    -- 4. Move to the calculated interception point and ALWAYS face the ball
    draw_point(target_x, target_y, true, {r=0.0, g=1.0, b=0.0}) -- Green point for the target

    move_to(robotId, team, {x = target_x, y = target_y})
    face_to(robotId, team, {x = bx, y = by})

    if dist < 0.2 then
        return true
    end
    return false
end

return receive_pass