-- mark: marks an enemy robot, standing between it and the ball to cut passes to it.
-- Parameters: robotId, team, targetId (the robot to mark, on the other team).
-- Terminates when the robot is on the marking spot (within SPOT_TOLERANCE); called every
-- tick it keeps following the target.
--
-- The marking spot is on the line target -> ball, MARK_DISTANCE from the target, so a pass
-- to it has to go through the marker. With the ball closer than that to the target, the
-- spot is the ball side of the target, as near as the robots can be without touching.
local mark = {}

local profile = require("utils.robot_profile")

local MARK_DISTANCE = 0.45   -- m, target to marking spot
local GAP = 0.05             -- m, free space kept between the two robot bodies
local SPOT_TOLERANCE = 0.05  -- m, distance to the spot that counts as there
local MIN_LENGTH = 0.001     -- m, guard for divisions by a length

--- The marking spot for a target at `target` with the ball at `ball`.
function mark.spot(target, ball)
    local dx, dy = ball.x - target.x, ball.y - target.y
    local length = math.max(math.sqrt(dx ^ 2 + dy ^ 2), MIN_LENGTH)
    local closest = 2 * profile.robot.radius + GAP
    local d = math.max(closest, math.min(MARK_DISTANCE, length - profile.touch_distance()))
    return { x = target.x + dx / length * d, y = target.y + dy / length * d }
end

local function is_done(robot, spot)
    return math.sqrt((robot.x - spot.x) ^ 2 + (robot.y - spot.y) ^ 2) <= SPOT_TOLERANCE
end

function mark.process(robotId, team, targetId)
    local robot = get_robot_state(robotId, team)
    local target = get_robot_state(targetId, 1 - team)
    local ball = get_ball_state()
    local spot = mark.spot(target, ball)

    if is_done(robot, spot) then
        return true
    end

    draw_point(spot.x, spot.y, true, { r = 1.0, g = 1.0, b = 0.0 })
    move_to(robotId, team, spot)
    face_to(robotId, team, { x = ball.x, y = ball.y })
    return false
end

return mark
