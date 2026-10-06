-- robot_profile: the active robot profile and what follows from it.
--
-- A profile (lua/profiles/<name>.lua) says what our robots and the ball can do:
-- speeds, turn rate, sizes, kick speed, ball deceleration. Every physical assumption
-- in the AI and the skills comes from here, so measuring a profile on real robots
-- (lua/tests/calibrate_profile.lua) is enough to adapt their decisions.
--
-- The profile is lua/profiles/active.lua's name, unless the entry script sets the global
-- ROBOT_PROFILE before requiring the AI (read the first time this module is required):
--     ROBOT_PROFILE = "real"
--     local ai = require("AI.main")

local M = {}

local name = rawget(_G, "ROBOT_PROFILE") or require("profiles.active")
local profile = require("profiles." .. name)

M.name = profile.name or name
M.robot = profile.robot
M.ball = profile.ball
M.kick = profile.kick
M.timing_noise = profile.timing_noise or 0.3
-- Values older profiles may lack.
M.robot.max_accel = M.robot.max_accel or 3.0
M.kick.one_touch_angle = M.kick.one_touch_angle or 0.0
M.kick.angle_error = M.kick.angle_error or 0.1
-- The opponents' motion: their own section when the profile has one, else our robot's.
M.opponent = setmetatable(profile.opponent or {}, { __index = M.robot })
-- How much of our line-up time opponents use to read a pass before it is kicked: 0 = they
-- follow the ball only, 1 = they start covering the receiver at once.
M.opponent.read_passes = rawget(M.opponent, "read_passes") or 0.0
-- guard_distance (m, optional): opponents that follow the ball stand this far in front of
-- it, towards their goal. nil: no assumption, they are raced from where they stand.

-- Ball speed fraction at which sliding turns into rolling (rolling without slipping).
local ROLL_FRACTION = 5 / 7

--- Robot center to ball center when the ball touches the kicker face, m.
function M.contact_distance()
    return M.robot.front_distance + M.ball.radius
end

--- Robot plus ball radius: closer than this the robot touches the ball, m.
function M.touch_distance()
    return M.robot.radius + M.ball.radius
end

--- Largest angle between the robot's heading and an incoming ball that still hits the
--- kicker face, rad (the ball's path through the robot center meets the face within
--- kicker_half_width).
function M.kicker_acceptance()
    return math.atan(M.robot.kicker_half_width, M.contact_distance())
end

--- How far a ball rolls until it stops, from `speed` (m/s; default: the kick speed), m.
--- @param speed? number
--- @return number
function M.roll_distance(speed)
    local v0 = speed or M.kick.speed
    local v1 = v0 * ROLL_FRACTION
    local slide = (v0 * v0 - v1 * v1) / (2 * M.ball.slide_decel)
    local roll = v1 * v1 / (2 * M.ball.roll_decel)
    return slide + roll
end

--- Estimated seconds for `robot` (x, y, orientation) to reach `point` and face it:
--- move_overhead + distance / cruise_speed + turn / turn_rate, with `motion` (default: our
--- robot profile; M.opponent for enemies).
--- With `moving` set and the robot's velocity known (vel_x, vel_y), its current motion counts:
--- moving towards the point saves part of the start-up overhead, moving away costs the
--- braking time (speed away / max_accel) on top.
--- @param robot { x: number, y: number, orientation: number, vel_x?: number, vel_y?: number }
--- @param point { x: number, y: number }
--- @param motion? table
--- @param moving? boolean
--- @return number
function M.time_to_reach(robot, point, motion, moving)
    motion = motion or M.robot
    local dx, dy = point.x - robot.x, point.y - robot.y
    local d = math.sqrt(dx * dx + dy * dy)
    local turn = math.atan(dy, dx) - robot.orientation
    while turn > math.pi do turn = turn - 2 * math.pi end
    while turn <= -math.pi do turn = turn + 2 * math.pi end
    local overhead = motion.move_overhead or 0
    local extra = 0
    if moving and robot.vel_x and d > 1e-6 then
        local towards = (robot.vel_x * dx + robot.vel_y * dy) / d
        if towards >= 0 then
            overhead = overhead * (1 - math.min(1, towards / motion.cruise_speed))
        else
            extra = -towards / motion.max_accel
        end
    end
    return overhead + extra + d / motion.cruise_speed + math.abs(turn) / motion.turn_rate
end

--- Seconds an enemy robot needs to reach `point`, counting its current motion.
--- @param robot RobotState
--- @param point { x: number, y: number }
--- @return number
function M.enemy_time_to_reach(robot, point)
    return M.time_to_reach(robot, point, M.opponent, true)
end

--- Seconds a ball kicked at `speed` (default: the kick speed) needs to roll `dist` m, or
--- math.huge if it stops first. Slides at slide_decel down to 5/7 of its speed, then rolls.
--- @param dist number
--- @param speed? number
--- @return number
function M.ball_travel_time(dist, speed)
    local v0 = speed or M.kick.speed
    local v1 = v0 * ROLL_FRACTION
    local slide_dist = (v0 * v0 - v1 * v1) / (2 * M.ball.slide_decel)
    local function time_at(v, a, s)
        local disc = v * v - 2 * a * s
        if disc < 0 then return nil end
        return (v - math.sqrt(disc)) / a
    end
    if dist <= slide_dist then
        return time_at(v0, M.ball.slide_decel, dist)
    end
    local t_roll = time_at(v1, M.ball.roll_decel, dist - slide_dist)
    if not t_roll then return math.huge end
    return (v0 - v1) / M.ball.slide_decel + t_roll
end

--- Probability that something taking `margin` s less than its rival happens first, given the
--- profile's timing noise: 0.5 at a tie, → 1 for a large positive margin.
--- @param margin number
--- @return number
function M.first_chance(margin)
    return 1.0 / (1.0 + math.exp(-margin / M.timing_noise))
end

return M
