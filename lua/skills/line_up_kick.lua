-- line_up_kick: gets behind the ball, facing `target`, and waits there without touching it.
-- Parameters: robotId, team, target {x, y} (m): the point the robot will kick towards.
-- Terminates when the robot is on the spot behind the ball (within SPOT_TOLERANCE) and
-- faces the kick direction (within FACING_TOLERANCE); kick_to_point then kicks at once.
--
-- For holding a kick: a passer that waits lined up keeps the pass a short step away while
-- the defender commits to the ball (draw and pass).
local line_up_kick = {}

local profile = require("utils.robot_profile")

local CLEARANCE = 0.0385       -- m, gap between robot and ball on the spot (kick_to_point's approach)
local SPOT_TOLERANCE = 0.03    -- m
local FACING_TOLERANCE = 0.1   -- rad
local MIN_LENGTH = 0.001       -- m, guard for divisions by a length

local function wrap(angle)
    while angle > math.pi do angle = angle - 2 * math.pi end
    while angle < -math.pi do angle = angle + 2 * math.pi end
    return angle
end

--- The spot behind the ball as seen from `target`.
function line_up_kick.spot(ball, target)
    local dx, dy = ball.x - target.x, ball.y - target.y
    local length = math.max(math.sqrt(dx ^ 2 + dy ^ 2), MIN_LENGTH)
    local offset = profile.touch_distance() + CLEARANCE
    return { x = ball.x + dx / length * offset, y = ball.y + dy / length * offset }
end

local function is_done(robot, ball, target)
    local spot = line_up_kick.spot(ball, target)
    local kick = math.atan(target.y - ball.y, target.x - ball.x)
    return math.sqrt((robot.x - spot.x) ^ 2 + (robot.y - spot.y) ^ 2) <= SPOT_TOLERANCE
        and math.abs(wrap(robot.orientation - kick)) <= FACING_TOLERANCE
end

function line_up_kick.process(robotId, team, target)
    local robot = get_robot_state(robotId, team)
    local ball = get_ball_state()

    if is_done(robot, ball, target) then
        return true
    end

    local spot = line_up_kick.spot(ball, target)
    face_to(robotId, team, target)
    -- move_to routes around the ball when the robot starts on the wrong side of it.
    move_to(robotId, team, spot)
    return false
end

return line_up_kick
