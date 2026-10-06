-- kick_to_point: lines the robot up behind the ball and kicks it flat towards `target`.
-- Parameters: robotId, team, target {x, y} (m).
-- Terminates when the robot is lined up and the ball is within the kicker's reach, or
-- reaches it before the next tick, issuing the kick on that tick.
local kick_to_point = {}

local APPROACH_OFFSET = 0.15        -- m, ball to approach point, behind the ball (robot radius 0.09 + ball 0.0215 + 0.0385 clearance)
local TOUCH_OFFSET = 0.045          -- m, ball to the move_direct target while touching
local LINE_TOLERANCE = 0.10         -- m, lateral distance to the kick line to start touching
local FACING_TOLERANCE = 0.1        -- rad, heading error to the kick direction to start touching
local CONTACT_DIST = 0.073 + 0.0215 -- m, robot center to ball center when the ball touches the kicker
local KICK_REACH = 0.01             -- m, how far in front of the kicker face a kick still fires
local KICKER_HALF_WIDTH = 0.05      -- m, lateral reach of the kicker face
local TICK = 1 / 60                 -- s, control period
local MIN_LENGTH = 0.001            -- m, guard for divisions by a length

-- Observation only: the phase chosen on the last tick, per robot, for tests and
-- debugging. Written every tick, never read by the skill.
kick_to_point.debug = {}

local function report(robotId, team, phase, reason)
    kick_to_point.debug[team] = kick_to_point.debug[team] or {}
    kick_to_point.debug[team][robotId] = { phase = phase, reason = reason }
end

local function wrap(angle)
    while angle > math.pi do angle = angle - 2 * math.pi end
    while angle < -math.pi do angle = angle + 2 * math.pi end
    return angle
end

-- Point `offset` behind the ball, on the line from `target` through the ball.
local function get_kick_point(ball, target, offset)
    local dx = ball.x - target.x
    local dy = ball.y - target.y
    local dist = math.max(math.sqrt(dx ^ 2 + dy ^ 2), MIN_LENGTH)
    return { x = ball.x + dx / dist * offset, y = ball.y + dy / dist * offset }
end

-- Heading error of the robot to the kick direction (ball to target), in [0, pi].
-- Not the bearing to the ball: near the ball a few cm of lateral offset change that
-- bearing by more than the tolerance while the kick direction is still right.
local function kick_heading_error(robot, ball, target)
    local kick_angle = math.atan(target.y - ball.y, target.x - ball.x)
    return math.abs(wrap(robot.orientation - kick_angle))
end

-- Returns whether the robot is lined up to kick, and why not.
local function check_lined_up(robot, ball, target)
    local dx = ball.x - target.x
    local dy = ball.y - target.y
    local length = math.sqrt(dx ^ 2 + dy ^ 2)
    if length < MIN_LENGTH then
        return false, "target_on_ball"
    end
    local dir_x, dir_y = dx / length, dy / length -- from the target through the ball, to behind it
    local rx, ry = robot.x - ball.x, robot.y - ball.y
    if dir_x * rx + dir_y * ry < 0 then
        return false, "behind_fail"
    end
    local lateral = math.abs(dir_x * ry - dir_y * rx)
    if lateral > LINE_TOLERANCE then
        return false, string.format("line_fail %.3f>%.3f", lateral, LINE_TOLERANCE)
    end
    local facing = kick_heading_error(robot, ball, target)
    if facing > FACING_TOLERANCE then
        return false, string.format("facing_fail %.3f>%.3f", facing, FACING_TOLERANCE)
    end
    return true, "lined_up"
end

-- True when the ball is within the kicker's reach, or reaches it before the next tick
-- at the current closing speed. A wider window (has_the_ball, 0.12 m) can end the
-- skill with the ball out of reach, and then the kick never fires.
local function is_ball_at_kicker(robot, ball)
    local cos_h, sin_h = math.cos(robot.orientation), math.sin(robot.orientation)
    local dx, dy = ball.x - robot.x, ball.y - robot.y
    local along = dx * cos_h + dy * sin_h
    local side = -dx * sin_h + dy * cos_h
    local closing = (robot.vel_x - ball.vel_x) * cos_h + (robot.vel_y - ball.vel_y) * sin_h
    return along > 0 and math.abs(side) <= KICKER_HALF_WIDTH
        and along - CONTACT_DIST <= KICK_REACH + math.max(closing, 0) * TICK
end

local function is_done(robot, ball, target)
    return check_lined_up(robot, ball, target) and is_ball_at_kicker(robot, ball)
end

function kick_to_point.process(robotId, team, target)
    local robot = get_robot_state(robotId, team)
    local ball = get_ball_state()
    draw_point(target.x, target.y, true, { r = 1.0, g = 0.0, b = 0.0 })

    if is_done(robot, ball, target) then
        report(robotId, team, "done", "ball_at_kicker")
        kickx(robotId, team)
        return true
    end

    local lined_up, reason = check_lined_up(robot, ball, target)
    if lined_up then
        report(robotId, team, "touch", reason)
        move_direct(robotId, team, get_kick_point(ball, target, TOUCH_OFFSET))
        return false
    end

    report(robotId, team, "approach", reason)
    local approach = get_kick_point(ball, target, APPROACH_OFFSET)
    draw_point(approach.x, approach.y)
    -- Turn to the kick direction while approaching: that is the heading the lined-up
    -- check needs, so the robot does not turn to the ball first and back later.
    face_to(robotId, team, target)
    move_to(robotId, team, approach)
    return false
end

return kick_to_point
