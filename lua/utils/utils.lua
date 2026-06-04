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
    local DISTANCE_THRESHOLD = 0.135
    
    -- The maximum angular spread of the dribbler mouth (in radians).
    -- 0.35 radians is roughly 20 degrees.
    local ANGLE_THRESHOLD = 0.5
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




-- Helper function to calculate Euclidean distance between a robot and the ball
function utils.getDistance(robotPos, ballPos)
    local dx = robotPos.x - ballPos.x
    local dy = robotPos.y - ballPos.y
    return math.sqrt(dx * dx + dy * dy)
end


--[[
    Assigns roles to robots based on their proximity to the ball.
    - Role 0: Always Goalkeeper (assigned to goalkeeperId if provided, otherwise none)
    - Role 1: Closest robot to the ball (excluding the goalkeeper)
    - Role 2: Second closest, and so on...
--]]
function utils.assign_roles(teamId, goalkeeperId)
    local roleMap = {}
    
    -- 1. Handle the goalkeeper rule (if an ID is provided)
    if goalkeeperId ~= nil then
        roleMap[0] = goalkeeperId
    end
    
    -- 2. Get the current ball position from the game state API
    local ball = get_ball_state()
    
    -- 3. Gather all field robots (skipping the goalkeeper) and calculate their distance
    local fieldRobots = {}
    local allRobots 
    
    if teamId == 0 then
        allRobots = get_blue_team_state()
    else
        allRobots = get_yellow_team_state()
    end

    for _, robot in ipairs(allRobots) do
        -- Skip the goalkeeper if one was specified
        if robot.id ~= goalkeeperId then
            local distance = utils.getDistance(robot, ball)
            table.insert(fieldRobots, { id = robot.id, dist = distance })
        end
    end
    
    -- 4. Sort the field robots by distance (closest first)
    table.sort(fieldRobots, function(a, b)
        return a.dist < b.dist
    end)
    
    -- 5. Map the sorted order to Role IDs starting from 1
    for roleId, robotData in ipairs(fieldRobots) do
        roleMap[roleId] = robotData.id
    end
    
    return roleMap
end

local BallMeta = {
    State = {
        stopped = "Stopped",
        moving = "Moving",
        unknown = "Unknown" -- Useful if the camera loses the ball
    }
}

--- Analyzes the ball's velocity to determine its meta-state
--- @return string - "Stopped", "Moving", or "Unknown"
function utils.get_ball_metastate()
    local ball = get_ball_state()
    
    -- If the ball isn't found on the field, return unknown
    if not ball then
        return BallMeta.State.unknown
    end

    -- Extract velocity (default to 0 if your API doesn't provide them when stopped)
    local vx = ball.vel_x or 0
    local vy = ball.vel_y or 0

    -- Calculate the total speed (magnitude of the velocity vector)
    local speed = math.sqrt(vx^2 + vy^2)

    -- Threshold in meters per second. Anything slower than this is considered "Stopped"
    local STOPPED_THRESHOLD = 0.05 

    if speed < STOPPED_THRESHOLD then
        return BallMeta.State.stopped
    else
        return BallMeta.State.moving
    end
end


return utils