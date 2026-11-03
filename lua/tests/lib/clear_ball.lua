-- clear_ball: a defender kicks the ball out of the field when it gets the chance.
--
-- Shared by the yellow test defenses (defense_2v2, defense_3v3). Each tick the defense asks
-- `clear_ball.chance(robot)`; while it is true the robot runs `clear_ball.process` instead of
-- its normal job (mark, guard). A chance is a ball within CHANCE_DIST of the robot, slower
-- than MAX_BALL_SPEED (not a shot or pass in full flight), with a kicking spot outside the
-- defense areas (only goalkeepers may enter them), and moving (MOVING_SPEED: a ball an
-- attacker pushes, or one rolling slowly). A resting ball is left to the normal defense.
--
-- The clear goes towards the nearest sideline and away from the defender's own goal, far
-- enough to leave the field. The line-up is loose (a clear only has to go roughly that way),
-- so a defender next to a ball that the attacker is pushing gets its kick in.
local profile = require("utils.robot_profile")

local M = {}

M.CHANCE_DIST = 0.5        -- m, robot centre to ball: close enough to go for the clear
M.MAX_BALL_SPEED = 1.5     -- m/s, faster balls are passes or shots in flight
M.MOVING_SPEED = 0.3       -- m/s, a ball at least this fast is moving (pushed or rolling)
M.BACK = 1.0               -- m, the clear also goes this far away from the defender's goal
M.OUT = 1.0                -- m, the target lies this far beyond the sideline
M.APPROACH_GAP = 0.03      -- m, ball to robot face at the approach spot
M.FACING_TOLERANCE = 0.25  -- rad, heading error to the clear direction to go in and kick
M.LINE_TOLERANCE = 0.04    -- m, lateral offset from the clear line to go in and kick
M.AREA_X, M.AREA_HALF_Y = 3.5, 1.0  -- m, the defense area fronts (|x| >= AREA_X, |y| <= AREA_HALF_Y)
M.AREA_ROOM = 0.05         -- m, extra room a defender keeps from the areas
M.FIELD_HALF_Y = 3.0       -- m

local function wrap(a)
    while a > math.pi do a = a - 2 * math.pi end
    while a < -math.pi do a = a + 2 * math.pi end
    return a
end

local function in_area(p, r)
    return math.abs(p.x) > M.AREA_X - r and math.abs(p.y) < M.AREA_HALF_Y + r
end

--- Where a defender defending the goal at `own_goal` clears the ball at `ball` to.
--- @param ball { x: number, y: number }
--- @param own_goal { x: number, y: number }
--- @return { x: number, y: number }
function M.target(ball, own_goal)
    local side = ball.y >= 0 and 1 or -1
    local away = own_goal.x >= 0 and -1 or 1
    return { x = ball.x + away * M.BACK, y = side * (M.FIELD_HALF_Y + M.OUT) }
end

--- The spot behind the ball, as seen from the clear target, `gap` from the ball's surface.
local function spot(ball, target, gap)
    local dx, dy = ball.x - target.x, ball.y - target.y
    local d = math.max(math.sqrt(dx * dx + dy * dy), 1e-6)
    local back = profile.contact_distance() + gap
    return { x = ball.x + dx / d * back, y = ball.y + dy / d * back }
end

--- Whether the defender `robot` has a chance to clear the ball now.
--- @param robot RobotState
--- @param own_goal { x: number, y: number }
--- @return boolean
function M.chance(robot, own_goal)
    local ball = get_ball_state()
    local speed = math.sqrt(ball.vel_x ^ 2 + ball.vel_y ^ 2)
    if speed > M.MAX_BALL_SPEED then return false end
    local d = math.sqrt((robot.x - ball.x) ^ 2 + (robot.y - ball.y) ^ 2)
    if d > M.CHANCE_DIST then return false end
    -- Only a moving ball (pushed by an attacker, or rolling slowly): a resting ball stays with
    -- whoever lines it up, as before.
    if speed < M.MOVING_SPEED then return false end
    local s = spot(ball, M.target(ball, own_goal), M.APPROACH_GAP)
    return not in_area(s, profile.robot.radius + M.AREA_ROOM)
        and not in_area(ball, profile.robot.radius + M.AREA_ROOM)
end

--- One tick of the clear for robot `id` of `team`.
function M.process(id, team, own_goal)
    local robot = get_robot_state(id, team)
    local ball = get_ball_state()
    local target = M.target(ball, own_goal)
    local heading = math.atan(target.y - ball.y, target.x - ball.x)
    local ux, uy = math.cos(heading), math.sin(heading)
    local rx, ry = robot.x - ball.x, robot.y - ball.y
    local behind = -(ux * rx + uy * ry)
    local lateral = math.abs(ux * ry - uy * rx)
    face_to(id, team, target)
    if behind > 0 and lateral <= M.LINE_TOLERANCE and math.abs(wrap(robot.orientation - heading)) <= M.FACING_TOLERANCE then
        move_direct(id, team, spot(ball, target, -0.03))   -- drive through the ball
        kickx(id, team)                                    -- fires when the ball reaches the face
        return
    end
    move_to(id, team, spot(ball, target, M.APPROACH_GAP))
end

return M
