local utils = {}

--- Checks if a specific robot currently possesses the ball.
--- @param robotId number
--- @param team number
--- @return boolean
function utils.has_the_ball(robotId, team)
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

return utils