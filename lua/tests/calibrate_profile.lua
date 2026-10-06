-- calibrate_profile: measures a robot and prints a robot profile (lua/profiles/<name>.lua).
--
-- Works on the simulator and on real robots (vision + radio): it counts control ticks
-- (60 Hz) instead of using simulator time, and moves relative to where the robot starts.
--
--   Simulator, headless:  ./target/debug/engine.exe --headless lua/tests/calibrate_profile.lua
--   Real robot (GUI):     load this file, place the robot with ~2.5 m free in front of it
--                         and the ball where the script asks, press play.
--
-- Parts (each prints its measurements):
--   1. drive: forward and back DRIVE_DISTANCES along the robot's heading (move_to)
--   2. turn:  TURN_ANGLES in place (face_to)
--   3. kick:  kick the ball straight ahead (kick_to_point) and follow it until it stops
-- At the end it prints the fitted profile, ready to save as lua/profiles/<name>.lua.
package.path = "lua/?.lua;" .. package.path
local kick_to_point = require("skills.kick_to_point")

local ROBOT_ID, TEAM = 0, 0
local PROFILE_NAME = "measured"
local DRIVE_DISTANCES = { 0.5, 1.0, 2.0 }        -- m, each driven forward and back
local TURN_ANGLES = { math.pi / 2, -math.pi / 2, math.pi } -- rad, relative turns
local TICK = 1 / 60                              -- s, control period
local ARRIVED_DIST = 0.05                        -- m, at the target
local ARRIVED_ANGLE = 0.05                       -- rad, facing the target heading
local STOPPED_SPEED = 0.1                        -- m/s, robot at rest
local STOPPED_OMEGA = 0.2                        -- rad/s, robot not turning
local MOVE_TIMEOUT = 8                           -- s, give up on one move
local BALL_AHEAD = 0.4                           -- m, where the ball is placed (or expected) in front of the robot
local KICK_TARGET_DIST = 4.0                     -- m, kick aim point ahead of the ball
local KICKED_SPEED = 0.5                         -- m/s, ball faster than this: it was kicked
local BALL_STOPPED = 0.05                        -- m/s, ball at rest again
local SETTLE_TICKS = 30                          -- ticks to wait between moves

local headless = rawget(_G, "sim") ~= nil
local robot_profile = require("utils.robot_profile") -- defaults (geometry) for the printout

local function wrap(a)
    while a > math.pi do a = a - 2 * math.pi end
    while a <= -math.pi do a = a + 2 * math.pi end
    return a
end

local function speed_of(s) return math.sqrt(s.vel_x ^ 2 + s.vel_y ^ 2) end

-- Steps run one after another; each is { start = fn(), tick = fn(ticks) -> done }.
local steps, step_i, step_ticks = {}, 0, 0
local drives, turns, kick = {}, {}, nil
local home -- start pose

local function add_settle()
    steps[#steps + 1] = { start = function() end, tick = function(t) return t >= SETTLE_TICKS end }
end

local function add_drive(dist, back)
    local target, label
    steps[#steps + 1] = {
        start = function()
            local h = home.orientation
            local d = back and 0 or dist
            target = { x = home.x + d * math.cos(h), y = home.y + d * math.sin(h) }
            label = string.format("%s %.1f m", back and "back" or "forward", dist)
        end,
        tick = function(t)
            local r = get_robot_state(ROBOT_ID, TEAM)
            move_to(ROBOT_ID, TEAM, target)
            face_to(ROBOT_ID, TEAM, { x = r.x + math.cos(home.orientation), y = r.y + math.sin(home.orientation) })
            local at = math.sqrt((r.x - target.x) ^ 2 + (r.y - target.y) ^ 2) <= ARRIVED_DIST
            if (at and speed_of(r) < STOPPED_SPEED) or t * TICK >= MOVE_TIMEOUT then
                local ok = at
                drives[#drives + 1] = { dist = dist, t = t * TICK, ok = ok }
                print(string.format("[calibrate] drive %-14s %.2f s%s", label, t * TICK, ok and "" or "  (TIMEOUT)"))
                return true
            end
        end,
    }
end

local function add_turn(angle)
    local heading
    steps[#steps + 1] = {
        start = function()
            heading = get_robot_state(ROBOT_ID, TEAM).orientation + angle
        end,
        tick = function(t)
            local r = get_robot_state(ROBOT_ID, TEAM)
            face_to(ROBOT_ID, TEAM, { x = r.x + math.cos(heading), y = r.y + math.sin(heading) })
            local err = math.abs(wrap(r.orientation - heading))
            if (err <= ARRIVED_ANGLE and math.abs(r.omega) < STOPPED_OMEGA) or t * TICK >= MOVE_TIMEOUT then
                turns[#turns + 1] = { angle = math.abs(angle), t = t * TICK, ok = err <= ARRIVED_ANGLE }
                print(string.format("[calibrate] turn %+.0f deg  %.2f s%s", math.deg(angle), t * TICK,
                    err <= ARRIVED_ANGLE and "" or "  (TIMEOUT)"))
                return true
            end
        end,
    }
end

local function add_kick()
    local aim, waiting_printed, samples, kicked_at, start_pos
    steps[#steps + 1] = {
        start = function()
            local h = home.orientation
            local bx, by = home.x + BALL_AHEAD * math.cos(h), home.y + BALL_AHEAD * math.sin(h)
            if headless then
                grsim.teleport_ball(bx, by)
            end
            aim = { x = bx + KICK_TARGET_DIST * math.cos(h), y = by + KICK_TARGET_DIST * math.sin(h) }
            samples = {}
        end,
        tick = function(t)
            local ball = get_ball_state()
            local r = get_robot_state(ROBOT_ID, TEAM)
            if not kicked_at then
                local d = math.sqrt((ball.x - r.x) ^ 2 + (ball.y - r.y) ^ 2)
                if d > 1.0 and not headless then
                    if not waiting_printed then
                        print(string.format("[calibrate] place the ball %.1f m in front of the robot", BALL_AHEAD))
                        waiting_printed = true
                    end
                    return false
                end
                if speed_of(ball) >= KICKED_SPEED then
                    kicked_at = t
                    start_pos = { x = ball.x, y = ball.y }
                else
                    kick_to_point.process(ROBOT_ID, TEAM, aim)
                    return t * TICK >= MOVE_TIMEOUT and (function() print("[calibrate] kick: TIMEOUT") return true end)()
                end
            end
            samples[#samples + 1] = { t = (t - kicked_at) * TICK, v = speed_of(ball), x = ball.x, y = ball.y }
            if speed_of(ball) < BALL_STOPPED and #samples > 5 then
                local dist = math.sqrt((ball.x - start_pos.x) ^ 2 + (ball.y - start_pos.y) ^ 2)
                kick = { samples = samples, dist = dist }
                print(string.format("[calibrate] kick: rolled %.2f m in %.2f s", dist, samples[#samples].t))
                return true
            end
            return (t - kicked_at) * TICK >= 15
        end,
    }
end

local MAX_RATE = 20.0 -- rad/s or m/s, cap when the time hardly grows with the amount (overhead-dominated)

-- Least squares t = t0 + x / rate; returns rate, t0 (rate capped at MAX_RATE).
local function fit_rate(pairs)
    local n, sx, st, sxx, sxt = 0, 0, 0, 0, 0
    for _, p in ipairs(pairs) do
        n, sx, st, sxx, sxt = n + 1, sx + p[1], st + p[2], sxx + p[1] * p[1], sxt + p[1] * p[2]
    end
    if n < 2 or n * sxx - sx * sx <= 0 then return nil, nil end
    local slope = (n * sxt - sx * st) / (n * sxx - sx * sx)
    local t0 = (st - slope * sx) / n
    if slope <= 1 / MAX_RATE then
        return MAX_RATE, st / n - sx / n / MAX_RATE
    end
    return 1 / slope, t0
end

-- Least-squares slope of v(t), as a positive deceleration.
local function decel(samples, from, to)
    local n, st, sv, stt, stv = 0, 0, 0, 0, 0
    for i = from, to do
        local s = samples[i]
        n = n + 1
        st, sv, stt, stv = st + s.t, sv + s.v, stt + s.t * s.t, stv + s.t * s.v
    end
    if n < 3 then return nil end
    local slope = (n * stv - st * sv) / (n * stt - st * st)
    return -slope
end

local function report()
    local drive_pairs, turn_pairs = {}, {}
    for _, d in ipairs(drives) do if d.ok then drive_pairs[#drive_pairs + 1] = { d.dist, d.t } end end
    for _, t in ipairs(turns) do if t.ok then turn_pairs[#turn_pairs + 1] = { t.angle, t.t } end end
    local cruise, move_overhead = fit_rate(drive_pairs)
    local turn_rate, turn_overhead = fit_rate(turn_pairs)

    local kick_speed, slide, roll, rolled
    if kick then
        local s = kick.samples
        local i_peak = 1
        for i = 1, math.min(#s, 10) do if s[i].v > s[i_peak].v then i_peak = i end end
        kick_speed = s[i_peak].v
        -- Sliding until 5/7 of the kick speed, rolling after (fits with fewer samples on real vision).
        local i_roll = i_peak
        while i_roll < #s and s[i_roll].v > kick_speed * 5 / 7 do i_roll = i_roll + 1 end
        slide = decel(s, i_peak, i_roll)
        roll = decel(s, i_roll, #s - 1)
        rolled = kick.dist
    end

    local p = robot_profile
    local function num(v, fallback) return string.format("%.3f", v or fallback) end
    print("[calibrate] ---------------------------------------------------------------")
    print(string.format("[calibrate] measured: drive %s m/s + %s s, turn %s rad/s + %s s, kick %s m/s, slide %s, roll %s m/s^2, rolled %s m",
        num(cruise, 0 / 0), num(move_overhead, 0 / 0), num(turn_rate, 0 / 0), num(turn_overhead, 0 / 0),
        num(kick_speed, 0 / 0), num(slide, 0 / 0), num(roll, 0 / 0), num(rolled, 0 / 0)))
    print("[calibrate] profile (save as lua/profiles/" .. PROFILE_NAME .. ".lua):")
    print(string.format([[
-- Robot profile measured by lua/tests/calibrate_profile.lua (%s).
return {
    name = "%s",
    robot = {
        radius = %.3f,%s
        front_distance = %.3f,%s
        kicker_half_width = %.3f,%s
        cruise_speed = %s,      -- m/s, once moving
        turn_rate = %s,         -- rad/s, once turning
        move_overhead = %s,     -- s, fixed cost of a move (drive fit; turn fit gave %s s)
        max_accel = %.1f,%s
    },
    ball = {
        radius = %.4f,%s
        slide_decel = %s,       -- m/s^2
        roll_decel = %s,        -- m/s^2
    },
    kick = {
        speed = %s,             -- m/s
        reach = %.3f,%s
        one_touch_angle = %.2f,%s
    },
    timing_noise = %.2f,%s
}]], headless and "simulator" or "real robot", PROFILE_NAME,
        p.robot.radius, "             -- copied (not measured)", p.robot.front_distance, "     -- copied (not measured)",
        p.robot.kicker_half_width, "  -- copied (not measured)",
        num(cruise, p.robot.cruise_speed), num(turn_rate, p.robot.turn_rate),
        num(move_overhead, p.robot.move_overhead or 0), num(turn_overhead, 0 / 0),
        p.robot.max_accel, "            -- copied (not measured)",
        p.ball.radius, "            -- copied (not measured)",
        num(slide, p.ball.slide_decel), num(roll, p.ball.roll_decel),
        num(kick_speed, p.kick.speed), p.kick.reach, "              -- copied (not measured)",
        p.kick.one_touch_angle, "     -- copied (not measured): test one-touch passes to set it",
        p.timing_noise, "            -- s, copied: how far off time estimates can be"))
end

-- Program
for _, d in ipairs(DRIVE_DISTANCES) do
    add_drive(d, false); add_settle()
    add_drive(d, true); add_settle()
end
for _, a in ipairs(TURN_ANGLES) do
    add_turn(a); add_settle()
end
add_kick()

local started = false
function process()
    if not started then
        if headless then
            grsim.teleport_robot(ROBOT_ID, TEAM, -3.0, -1.0, 0.0)
            grsim.teleport_ball(4.0, 2.5) -- out of the way until the kick part
            started = true
            return
        end
        started = true
    end
    if not home then
        home = get_robot_state(ROBOT_ID, TEAM)
        step_i, step_ticks = 1, 0
        steps[1].start()
        print(string.format("[calibrate] robot %d team %d, profile '%s' defaults; %s", ROBOT_ID, TEAM,
            robot_profile.name, headless and "simulator" or "GUI"))
    end
    if step_i > #steps then
        return
    end
    step_ticks = step_ticks + 1
    if steps[step_i].tick(step_ticks) then
        step_i, step_ticks = step_i + 1, 0
        if step_i > #steps then
            report()
            if headless then sim.finish(true) end
        else
            steps[step_i].start()
        end
    end
end
