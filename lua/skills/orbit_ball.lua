-- orbit_ball: circles the ball to get behind it as seen from `target`, without stopping.
-- Parameters: robotId, team, target {x, y} (m): the point the robot will kick towards.
-- Terminates when the robot's bearing around the ball is within DONE_ANGLE of the spot
-- behind the ball (then kick_to_point can line up with a short approach).
--
-- Faster than driving to a point behind the ball with move_to when the robot is on the
-- wrong side: move_to routes around the ball as an obstacle and stops at each waypoint.
local orbit_ball = {}

local profile = require("utils.robot_profile")

local CLEARANCE = 0.0885  -- m, gap between robot and ball while circling it
local DONE_ANGLE = 0.6    -- rad, bearing around the ball from "behind" that counts as done
local STEP = 0.7          -- rad, how far ahead on the circle the robot aims each tick

local function wrap(angle)
    while angle > math.pi do angle = angle - 2 * math.pi end
    while angle < -math.pi do angle = angle + 2 * math.pi end
    return angle
end

-- Signed angle around the ball from the robot to the spot behind the ball (as seen from target).
local function angle_to_behind(robot, ball, target)
    local behind = math.atan(ball.y - target.y, ball.x - target.x)
    local bearing = math.atan(robot.y - ball.y, robot.x - ball.x)
    return wrap(behind - bearing), bearing
end

local function is_done(robot, ball, target)
    local around = angle_to_behind(robot, ball, target)
    return math.abs(around) <= DONE_ANGLE
end

--- Whether the robot is far enough around the ball that orbiting is worth it.
--- @param robot RobotState
--- @param ball BallState
--- @param target { x: number, y: number }
--- @return boolean
function orbit_ball.needed(robot, ball, target)
    return not is_done(robot, ball, target)
end

function orbit_ball.process(robotId, team, target)
    local robot = get_robot_state(robotId, team)
    local ball = get_ball_state()

    if is_done(robot, ball, target) then
        return true
    end

    local around, bearing = angle_to_behind(robot, ball, target)
    local aim = bearing + math.max(-STEP, math.min(STEP, around))
    local radius = profile.touch_distance() + CLEARANCE
    -- Face the kick direction already, so the line-up after the orbit is short.
    face_to(robotId, team, target)
    move_direct(robotId, team, { x = ball.x + radius * math.cos(aim), y = ball.y + radius * math.sin(aim) })
    return false
end

return orbit_ball
