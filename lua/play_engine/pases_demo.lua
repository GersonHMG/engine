local pases = {}

local pase = require("play.pass")

-- Keep track of the current roles inside the module
pases.currentRoles = {}

-- Helper function to calculate Euclidean distance between a robot and the ball
local function getDistance(robotPos, ballPos)
    local dx = robotPos.x - ballPos.x
    local dy = robotPos.y - ballPos.y
    return math.sqrt(dx * dx + dy * dy)
end

--[[
    Assigns roles to robots based on their proximity to the ball.
    - Role 0: Always Goalkeeper (Robot 0)
    - Role 1: Closest robot to the ball (excluding Robot 0)
    - Role 2: Second closest, and so on...
--]]
function pases.getRoleAssignment(teamId)
    local roleMap = {}
    
    -- 1. Hardcode the goalkeeper rule
    roleMap[0] = 0
    
    -- 2. Get the current ball position from the game state API
    local ball = get_ball_state() 
    
    -- 3. Gather all field robots (skipping robot 0) and calculate their distance to the ball
    local fieldRobots = {}
    local allRobots -- Declared out here so it is properly scoped for both blocks
    
    if teamId == 0 then
        allRobots = get_blue_team_state()
    else
        allRobots = get_yellow_team_state()
    end

    for _, robot in ipairs(allRobots) do
        if robot.id ~= 0 then
            local distance = getDistance(robot, ball)
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

function pases.process(team)
    -- Initialize roles on the very first frame/execution
    if not pases.currentRoles[1] then
        pases.currentRoles = pases.getRoleAssignment(team)
    end

    -- Use the stored role mappings (Role 1 passes to Role 2)
    local passerId = pases.currentRoles[1]
    local receiverId = pases.currentRoles[2]

    -- Safety check: ensure we actually have enough field robots to execute a pass
    if passerId and receiverId then
        if pase.process(passerId, team, receiverId) then
            -- The pass was successful! Recalculate and update the role mappings.
            pases.currentRoles = pases.getRoleAssignment(team)
        end
    end
end

return pases