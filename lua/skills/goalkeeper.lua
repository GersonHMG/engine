-- goalkeeper: guards `goal`, standing on the goal line between the ball and the goal.
-- Parameters: robotId, team, goal {x, y} (m): the center of the goal to defend.
-- Terminates when the robot is on the guard spot (within SPOT_TOLERANCE); called every
-- tick it keeps tracking the ball.
--
-- The guard spot is where the line ball -> goal center crosses a line LINE_OFFSET in front
-- of the goal line, clamped so the robot stays between the posts. move_direct: the path
-- planner treats the defense area as an obstacle.
local goalkeeper = {}

local profile = require("utils.robot_profile")

local GOAL_HALF_WIDTH = 0.5  -- m, SSL division B goal (1.0 m wide)
local LINE_OFFSET = 0.15     -- m, in front of the goal line, towards the field
local SPOT_TOLERANCE = 0.03  -- m
local MIN_LENGTH = 0.001     -- m, guard for divisions by a length

--- The guard spot for the ball at `ball` and the goal centered at `goal`.
function goalkeeper.spot(ball, goal)
    local inward = goal.x > 0 and -1 or 1   -- from the goal line towards the field
    local line_x = goal.x + inward * LINE_OFFSET
    local dx = ball.x - goal.x
    local y = goal.y
    if math.abs(dx) > MIN_LENGTH then
        y = goal.y + (ball.y - goal.y) * (line_x - goal.x) / dx
    end
    local limit = GOAL_HALF_WIDTH - profile.robot.radius
    return { x = line_x, y = math.max(goal.y - limit, math.min(goal.y + limit, y)) }
end

local function is_done(robot, spot)
    return math.sqrt((robot.x - spot.x) ^ 2 + (robot.y - spot.y) ^ 2) <= SPOT_TOLERANCE
end

function goalkeeper.process(robotId, team, goal)
    local robot = get_robot_state(robotId, team)
    local ball = get_ball_state()
    local spot = goalkeeper.spot(ball, goal)

    face_to(robotId, team, { x = ball.x, y = ball.y })
    if is_done(robot, spot) then
        return true
    end

    move_direct(robotId, team, spot)
    return false
end

return goalkeeper
