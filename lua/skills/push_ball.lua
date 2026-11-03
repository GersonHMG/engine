-- push_ball: pushes the ball along the ground towards `target` with the kicker face.
-- Parameters: robotId, team, target {x, y} (m), speed (m/s, optional).
--
-- There is no dribbler in the simulator, but a robot driving straight at a constant speed
-- keeps a resting ball on its face: measured with sessions' push probe, the gap stays at the
-- contact distance (0.0945 m) up to 1.5 m/s. So the skill first lines up behind the ball,
-- facing the target (like kick_to_point, with looser tolerances: no kick has to leave
-- along the heading), then drives through the ball at `speed`, steering back onto the
-- line ball -> target.
--
-- Velocity commands bypass utils/area_guard, so the push itself stops (and hands the robot
-- to a guarded move) when the point where it would stop touches a defense area.
local push_ball = {}

local profile = require("utils.robot_profile")
local area_guard = require("utils.area_guard")

push_ball.SPEED = 1.0                -- m/s, default push speed (ball stays on the face up to 1.5)
local APPROACH_CLEARANCE = 0.0385    -- m, gap between robot and ball at the approach point
local LINE_TOLERANCE = 0.03          -- m, lateral distance to the push line to start pushing
local FACING_TOLERANCE = 0.1         -- rad, heading error to start pushing
local CONTACT_SLACK = 0.06           -- m, how far from contact the robot may start pushing
local LATERAL_GAIN = 4.0             -- 1/s, steering back onto the line
local TURN_GAIN = 4.0                -- 1/s, heading correction
local LOOKAHEAD = 0.15               -- m beyond the robot checked against the areas while pushing
local MIN_LENGTH = 0.001

local function wrap(angle)
    while angle > math.pi do angle = angle - 2 * math.pi end
    while angle < -math.pi do angle = angle + 2 * math.pi end
    return angle
end

--- With `brake`, the push slows down at the ball's own rolling deceleration so the ball
--- stops at `target` instead of rolling on by speed² / (2·roll_decel) (0.5 m at 1 m/s).
--- @return boolean pushing  -- true while the robot drives the ball (false while lining up)
function push_ball.process(robotId, team, target, speed, brake)
    speed = speed or push_ball.SPEED
    local robot = get_robot_state(robotId, team)
    local ball = get_ball_state()
    local dx, dy = target.x - ball.x, target.y - ball.y
    local length = math.max(math.sqrt(dx * dx + dy * dy), MIN_LENGTH)
    local ux, uy = dx / length, dy / length
    local heading = math.atan(uy, ux)
    draw_point(target.x, target.y, true, { r = 1.0, g = 0.5, b = 0.0 })

    local rx, ry = robot.x - ball.x, robot.y - ball.y
    local behind = -(ux * rx + uy * ry)           -- > 0: robot behind the ball
    local lateral = ux * ry - uy * rx             -- signed distance to the push line
    local facing = wrap(heading - robot.orientation)
    local contact = profile.contact_distance()

    local lined_up = behind > 0 and math.abs(lateral) <= LINE_TOLERANCE
        and math.abs(facing) <= FACING_TOLERANCE and behind <= contact + CONTACT_SLACK
    if lined_up then
        local ahead = { x = robot.x + ux * LOOKAHEAD, y = robot.y + uy * LOOKAHEAD }
        if area_guard.touched(ahead, area_guard.MARGIN) then
            move_to(robotId, team, { x = robot.x, y = robot.y })  -- guarded: brake here
            return false
        end
        if brake then
            local remaining = math.max(0.0, ux * dx + uy * dy)
            speed = math.min(speed, math.sqrt(2 * profile.ball.roll_decel * remaining))
        end
        -- Field-frame velocity: along the line, plus steering back onto it.
        local vx = speed * ux - LATERAL_GAIN * lateral * (-uy)
        local vy = speed * uy - LATERAL_GAIN * lateral * ux
        local c, s = math.cos(robot.orientation), math.sin(robot.orientation)
        send_velocity(robotId, team, c * vx + s * vy, -s * vx + c * vy, TURN_GAIN * facing)
        return true
    end

    local offset = profile.touch_distance() + APPROACH_CLEARANCE
    local approach = { x = ball.x - ux * offset, y = ball.y - uy * offset }
    if behind > 0 and math.abs(lateral) <= LINE_TOLERANCE and math.abs(facing) <= FACING_TOLERANCE then
        -- On the line, facing: close in on the ball.
        move_direct(robotId, team, { x = ball.x - ux * contact, y = ball.y - uy * contact })
        return false
    end
    face_to(robotId, team, target)
    move_to(robotId, team, approach)
    return false
end

return push_ball
