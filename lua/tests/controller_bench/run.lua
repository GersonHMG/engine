-- controller_bench/run.lua — runs one controller on every robot profile × test and
-- prints the metrics. The baseline is controllers/bang_bang.lua; see README.md.
--
--   ./target/debug/engine.exe --headless --max-time 600 lua/tests/controller_bench/run.lua
--
-- Environment variables:
--   CB_CONTROLLER  controllers/<name>.lua (default: bang_bang)
--   CB_ROBOTS      comma list of robots/<name>.toml (default: perfect_robot,one_bad_wheel,bad_torque)
--   CB_TESTS       comma list of tests (default: go_to_point,square,kick_to_point)
--   CB_CONDITION   ideal | realistic (default: realistic)
--
-- The simulator is deterministic (vision noise uses a fixed seed), so one run is enough:
-- any change in the numbers comes from the change you made.
package.path = "lua/?.lua;" .. package.path

local CONTROLLER = os.getenv("CB_CONTROLLER") or "bang_bang"
local controller = require("tests.controller_bench.controllers." .. CONTROLLER).new()

local RESULTS_DIR = "lua/tests/controller_bench/results/"
local ID, TEAM = 0, 0
local HOLD_TICKS = 30        -- ticks the robot is held at the start before each test
local POS_TOL = 0.02         -- m, "on the point"
local ANG_TOL = 0.05         -- rad, "facing the heading"
local REACH_TOL = 0.05       -- m, "got there" (first time), with REACH_ANG
local REACH_ANG = 0.1        -- rad
local HOLD_WINDOW = 1.0      -- s, the hold error is the RMS over the last HOLD_WINDOW of a test
local BALL_PARK = { x = 4.0, y = -2.7 }

local function list(env, default)
    local out = {}
    for name in (os.getenv(env) or default):gmatch("[^,%s]+") do out[#out + 1] = name end
    return out
end
local ROBOTS = list("CB_ROBOTS", "perfect_robot,one_bad_wheel,bad_torque")
local TEST_NAMES = list("CB_TESTS", "go_to_point,square,kick_to_point")
local CONDITION = os.getenv("CB_CONDITION") or "realistic"

local CONDITIONS = {
    ideal = { "sim vision position_noise 0", "sim vision angle_noise 0", "sim vision delay 0", "sim radio delay 0" },
    realistic = { "sim vision position_noise 0.0015", "sim vision angle_noise 0.002",
        "sim vision delay 0.033", "sim radio delay 0.033" },
}
assert(CONDITIONS[CONDITION], "CB_CONDITION must be ideal or realistic")
-- Override single values of the condition: CB_POSITION_NOISE (m), CB_ANGLE_NOISE (rad),
-- CB_VISION_DELAY (s), CB_RADIO_DELAY (s). They show in the condition's name.
local CONDITION_LINES = CONDITIONS[CONDITION]
do
    local overrides = {
        { "CB_POSITION_NOISE", "sim vision position_noise" }, { "CB_ANGLE_NOISE", "sim vision angle_noise" },
        { "CB_VISION_DELAY", "sim vision delay" }, { "CB_RADIO_DELAY", "sim radio delay" },
    }
    for _, o in ipairs(overrides) do
        local v = os.getenv(o[1])
        if v then
            assert(tonumber(v), o[1] .. " must be a number")
            local lines = CONDITION_LINES
            for i = #lines, 1, -1 do
                if lines[i]:find(o[2], 1, true) == 1 then table.remove(lines, i) end
            end
            lines[#lines + 1] = o[2] .. " " .. v
            CONDITION = CONDITION .. "+" .. o[1]:sub(4):lower() .. "=" .. v
        end
    end
end

-- ── helpers ────────────────────────────────────────────────────────────────

local function wrap(a)
    while a > math.pi do a = a - 2 * math.pi end
    while a <= -math.pi do a = a + 2 * math.pi end
    return a
end
local function dist(a, b) return math.sqrt((a.x - b.x) ^ 2 + (a.y - b.y) ^ 2) end
local function deg(r) return r * 180 / math.pi end

--- Rest-to-rest time over `d` with speed limit v and acceleration a.
local function trapezoid(d, v, a)
    if d <= 0 then return 0 end
    if d >= v * v / a then return d / v + v / a end
    return 2 * math.sqrt(d / a)
end

--- Distance from p to the segment a-b.
local function seg_dist(p, a, b)
    local abx, aby = b.x - a.x, b.y - a.y
    local l2 = abx * abx + aby * aby
    local t = l2 > 0 and math.max(0, math.min(1, ((p.x - a.x) * abx + (p.y - a.y) * aby) / l2)) or 0
    return dist(p, { x = a.x + t * abx, y = a.y + t * aby })
end

local function console(line)
    local out = sim.console(line)
    for _, l in ipairs(out) do
        if l:lower():find("^error") then error("console `" .. line .. "`: " .. l) end
    end
    return out
end

--- `name = value` lines of a console listing, as { name = number }.
local function read_values(line)
    local v = {}
    for _, l in ipairs(console(line)) do
        local name, value = l:match("^%s*([%w_%.]+)%s*=%s*([-+%d%.eE]+)")
        if name then v[name] = tonumber(value) end
    end
    return v
end

--- RMS position and heading error over the last HOLD_WINDOW: how still the robot holds
--- the pose once there (a controller that keeps hunting around the point shows here).
local function hold_errors(samples, T, H, t_end)
    local n, pe, ae = 0, 0, 0
    for _, s in ipairs(samples) do
        if s.t >= t_end - HOLD_WINDOW then
            n, pe = n + 1, pe + dist(s, T) ^ 2
            if H then ae = ae + wrap(s.th - H) ^ 2 end
        end
    end
    if n == 0 then return nil, nil end
    return math.sqrt(pe / n), H and math.sqrt(ae / n) or nil
end

local function peak_speed(samples)
    local v = 0
    for _, s in ipairs(samples) do v = math.max(v, math.sqrt(s.vx ^ 2 + s.vy ^ 2)) end
    return v
end

-- ── tests ──────────────────────────────────────────────────────────────────
-- Every test starts with blue 0 at rest at (0, 0) facing +x. A test has
--   duration                 s, the most it can run
--   ball                     where the ball is put (default: out of the way)
--   init(ctx)                once, when it starts
--   step(ctx, t, truth)      every tick: returns the pose {x, y, theta} to drive to, or nil = done
--   metrics(ctx, samples)    the result table
--   columns                  { {key, header, format, scale} } printed and written to the CSV
--   primary                  the column that ranks the test (lower = better)

local P = controller.params

local TESTS = {}

TESTS.go_to_point = {
    describe = "(0,0,0°) → (2.0, 1.0, 90°)",
    duration = 6.0,
    target = { x = 2.0, y = 1.0, theta = math.pi / 2 },
    init = function() end,
    step = function(ctx, t) return ctx.test.target end,
    metrics = function(ctx, samples)
        local T = ctx.test.target
        local m = {}
        local settled
        for i = #samples, 1, -1 do
            local s = samples[i]
            if dist(s, T) > POS_TOL or math.abs(wrap(s.th - T.theta)) > ANG_TOL then break end
            settled = s.t
        end
        -- Inside the tolerance only at the very end is not settled: it has to stay 0.25 s.
        m.t_settle = (settled and settled <= ctx.test.duration - 0.25) and settled or nil
        for _, s in ipairs(samples) do
            if dist(s, T) <= REACH_TOL and math.abs(wrap(s.th - T.theta)) <= REACH_ANG then m.t_reach = s.t break end
        end
        m.t_plan = math.max(trapezoid(dist({ x = 0, y = 0 }, T), P.v_max, P.a_max),
            trapezoid(math.abs(T.theta), P.omega_max, P.alpha_max))
        local d = dist({ x = 0, y = 0 }, T)
        local ux, uy = T.x / d, T.y / d
        local os, dev = 0, 0
        for _, s in ipairs(samples) do
            os = math.max(os, (s.x - T.x) * ux + (s.y - T.y) * uy)
            dev = math.max(dev, math.abs(s.x * uy - s.y * ux))
        end
        m.overshoot, m.path_dev = os, dev
        m.hold_err, m.hold_ang = hold_errors(samples, T, T.theta, ctx.test.duration)
        m.v_peak = peak_speed(samples)
        return m
    end,
    columns = {
        { "cost", "cost", "%.2f" }, { "t_reach", "reach_s", "%.2f" }, { "t_settle", "settle_s", "%.2f" }, { "t_plan", "plan_s", "%.2f" },
        { "overshoot", "os_mm", "%.0f", 1000 }, { "path_dev", "dev_mm", "%.0f", 1000 },
        { "hold_err", "hold_mm", "%.1f", 1000 }, { "hold_ang", "hold_deg", "%.2f", 180 / math.pi },
        { "v_peak", "v_peak", "%.2f" },
    },
    primary = "t_reach",
    -- s: time to reach (10 if never) + how far it hunts once there (50 mm = 1 s, at most 10).
    cost = function(m) return (m.t_reach or 10) + math.min(m.hold_err or 0.5, 0.5) / 0.05 end,
}

local CORNER_STILL = 0.3 -- m/s, a corner counts once the robot is within REACH_TOL of it and this slow
local SQUARE = { { x = 1.5, y = 0 }, { x = 1.5, y = 1.5 }, { x = 0, y = 1.5 }, { x = 0, y = 0 } }
TESTS.square = {
    describe = "1.5 m square (0,0)→(1.5,0)→(1.5,1.5)→(0,1.5)→(0,0), facing +x, brake at each corner",
    duration = 20.0,
    hold_after = 1.0, -- s held on the last corner, for the final error
    init = function(ctx) ctx.corner, ctx.arrivals = 1, {} end,
    step = function(ctx, t, truth)
        local c = SQUARE[ctx.corner]
        if not c then
            if t >= ctx.arrivals[#SQUARE] + ctx.test.hold_after then return nil end
            return { x = 0, y = 0, theta = 0 }
        end
        if truth and dist(truth, c) <= REACH_TOL and math.sqrt(truth.vel_x ^ 2 + truth.vel_y ^ 2) <= CORNER_STILL then
            ctx.arrivals[ctx.corner] = t
            ctx.corner = ctx.corner + 1
            c = SQUARE[ctx.corner] or c
        end
        return { x = c.x, y = c.y, theta = 0 }
    end,
    metrics = function(ctx, samples)
        local m = { corners = #ctx.arrivals }
        m.t_lap = #ctx.arrivals == #SQUARE and ctx.arrivals[#SQUARE] or nil
        local plan = 0
        for i = 1, #SQUARE do plan = plan + trapezoid(1.5, P.v_max, P.a_max) end
        m.t_plan = plan
        local sq, n, worst, head = 0, 0, 0, 0
        for _, s in ipairs(samples) do
            local e = math.huge
            for i = 1, #SQUARE do e = math.min(e, seg_dist(s, SQUARE[i == 1 and #SQUARE or i - 1], SQUARE[i])) end
            sq, n, worst = sq + e * e, n + 1, math.max(worst, e)
            head = math.max(head, math.abs(wrap(s.th)))
        end
        m.path_rms, m.path_max, m.heading_max = math.sqrt(sq / math.max(n, 1)), worst, head
        -- Overshoot past each corner along the edge that arrives at it.
        local os = 0
        for i = 1, #SQUARE do
            local a, c = SQUARE[i == 1 and #SQUARE or i - 1], SQUARE[i]
            local l = dist(a, c)
            local ux, uy = (c.x - a.x) / l, (c.y - a.y) / l
            local from = ctx.arrivals[i - 1] or 0
            local to = ctx.arrivals[i + 1] or math.huge
            for _, s in ipairs(samples) do
                if s.t >= from and s.t <= to then os = math.max(os, (s.x - c.x) * ux + (s.y - c.y) * uy) end
            end
        end
        m.corner_os = os
        local t_end = samples[#samples] and samples[#samples].t or 0
        if m.t_lap then m.hold_err = hold_errors(samples, SQUARE[#SQUARE], nil, t_end) end
        m.v_peak = peak_speed(samples)
        return m
    end,
    columns = {
        { "cost", "cost", "%.2f" }, { "t_lap", "lap_s", "%.2f" }, { "t_plan", "plan_s", "%.2f" }, { "corners", "corners", "%d" },
        { "path_rms", "rms_mm", "%.0f", 1000 }, { "path_max", "max_mm", "%.0f", 1000 },
        { "corner_os", "os_mm", "%.0f", 1000 }, { "heading_max", "head_deg", "%.1f", 180 / math.pi },
        { "hold_err", "hold_mm", "%.1f", 1000 }, { "v_peak", "v_peak", "%.2f" },
    },
    primary = "t_lap",
    -- s: lap time; if unfinished, 20 + 5 per corner not reached.
    cost = function(m) return m.t_lap or (20 + 5 * (#SQUARE - (m.corners or 0))) end,
}

local KICK_BALL = { x = 1.0, y = 0.5 }
local KICK_TARGET = { x = 3.0, y = -1.0 }
local BEHIND = 0.20       -- m, ball center to robot center at the approach point
local LINE_TOL = 0.02     -- m, lateral offset from the kick line to start pushing
local FACE_TOL = 0.03     -- rad, heading error to start pushing
local STILL = 0.1         -- m/s, "stopped" at the approach point
local ALIGN_GIVE_UP = 5.0 -- s, push anyway after this long without lining up (align_s = NO)
local TRACK_TIME = 2.5    -- s, ball followed after the kick
local DIR_AFTER = 0.3     -- s after the kick, ball direction measured from the kick point
TESTS.kick_to_point = {
    describe = "ball (1.0, 0.5) → target (3.0, -1.0): line up 0.2 m behind, push, kick (kick_x_speed)",
    duration = 12.0,
    ball = KICK_BALL,
    init = function(ctx)
        local dx, dy = KICK_TARGET.x - KICK_BALL.x, KICK_TARGET.y - KICK_BALL.y
        local l = math.sqrt(dx * dx + dy * dy)
        ctx.u = { x = dx / l, y = dy / l }
        ctx.heading = math.atan(dy, dx)
        ctx.phase, ctx.ball_track = "approach", {}
        sim.events()
    end,
    step = function(ctx, t, truth)
        for _, e in ipairs(sim.events()) do
            if e.kind == "kick" and not ctx.t_kick then
                ctx.t_kick, ctx.phase = t, "after"
                ctx.stop_at = truth and { x = truth.x, y = truth.y, theta = truth.orientation }
            end
        end
        local b = get_ball_state()
        if ctx.phase == "after" then
            ctx.ball_track[#ctx.ball_track + 1] = { t = t - ctx.t_kick, x = b.x, y = b.y }
            if t - ctx.t_kick >= TRACK_TIME then return nil end
            return ctx.stop_at
        end
        local u, h = ctx.u, ctx.heading
        if ctx.phase == "approach" then
            local goal = { x = KICK_BALL.x - u.x * BEHIND, y = KICK_BALL.y - u.y * BEHIND, theta = h }
            if truth then
                local rx, ry = truth.x - KICK_BALL.x, truth.y - KICK_BALL.y
                local lateral = math.abs(rx * u.y - ry * u.x)
                local speed = math.sqrt(truth.vel_x ^ 2 + truth.vel_y ^ 2)
                if lateral <= LINE_TOL and math.abs(wrap(truth.orientation - h)) <= FACE_TOL
                    and speed <= STILL and dist(truth, goal) <= REACH_TOL then
                    ctx.phase, ctx.t_aligned = "push", t
                elseif t >= ALIGN_GIVE_UP then
                    ctx.phase = "push"
                end
            end
            return goal
        end
        -- push: drive through the ball along the kick line, kicker armed.
        kickx(ID, TEAM)
        return { x = KICK_BALL.x + u.x * 0.05, y = KICK_BALL.y + u.y * 0.05, theta = h }
    end,
    metrics = function(ctx)
        local m = { t_aligned = ctx.t_aligned, t_kick = ctx.t_kick, kicked = ctx.t_kick and 1 or 0 }
        local track = ctx.ball_track
        if #track > 1 then
            local p0 = track[1]
            local p1 = track[#track]
            for _, s in ipairs(track) do
                if s.t >= DIR_AFTER then p1 = s break end
            end
            local dir = math.atan(p1.y - p0.y, p1.x - p0.x)
            m.dir_err = math.abs(wrap(dir - ctx.heading))
            local miss = math.huge
            for _, s in ipairs(track) do miss = math.min(miss, dist(s, KICK_TARGET)) end
            m.miss = miss
            m.ball_speed = dist(p0, p1) / math.max(p1.t - p0.t, 1e-6)
        end
        return m
    end,
    columns = {
        { "cost", "cost", "%.2f" }, { "kicked", "kicked", "%d" }, { "t_aligned", "align_s", "%.2f" }, { "t_kick", "kick_s", "%.2f" },
        { "dir_err", "dir_deg", "%.2f", 180 / math.pi }, { "miss", "miss_mm", "%.0f", 1000 },
        { "ball_speed", "ball_v", "%.2f" },
    },
    primary = "miss",
    -- s-equivalent: half the time to kick + the miss (100 mm = 1, at most 10); 15 without a kick.
    cost = function(m)
        if not m.t_kick or not m.miss then return 15 end
        return m.t_kick / 2 + math.min(m.miss, 1.0) / 0.1
    end,
}

for _, name in ipairs(TEST_NAMES) do assert(TESTS[name], "unknown test '" .. name .. "'") end

-- ── printing / CSV ─────────────────────────────────────────────────────────

local function cell(m, col)
    local v = m[col[1]]
    if v == nil then
        local timed = { t_settle = 1, t_reach = 1, t_lap = 1, t_kick = 1, t_aligned = 1 }
        return timed[col[1]] and "NO" or "-"
    end
    return string.format(col[3], v * (col[4] or 1))
end

local function print_header(test_name)
    local test = TESTS[test_name]
    print(string.format("\n== %s · %s · controller %s · %s ==", test_name, test.describe, CONTROLLER, CONDITION))
    local parts = { string.format("%-14s", "robot") }
    for _, c in ipairs(test.columns) do parts[#parts + 1] = string.format("%9s", c[2]) end
    print(table.concat(parts, " "))
end

local function print_row(robot, test_name, m)
    local parts = { string.format("%-14s", robot) }
    for _, c in ipairs(TESTS[test_name].columns) do parts[#parts + 1] = string.format("%9s", cell(m, c)) end
    print(table.concat(parts, " "))
end

local function write_csv(path, results)
    local file = io.open(path, "w")
    if not file then print("could not write " .. path) return end
    file:write("controller,condition,robot,test,metric,value\n")
    for _, test_name in ipairs(TEST_NAMES) do
        for _, robot in ipairs(ROBOTS) do
            local m = results[test_name][robot]
            for _, c in ipairs(TESTS[test_name].columns) do
                local v = m[c[1]]
                file:write(string.format("%s,%s,%s,%s,%s,%s\n", CONTROLLER, CONDITION, robot, test_name, c[2],
                    v and string.format("%.6g", v * (c[4] or 1)) or ""))
            end
        end
    end
    file:close()
    print("wrote " .. path)
end

--- { "robot|test|header" = number } from a CSV written by write_csv, or nil.
local function read_csv(path)
    local file = io.open(path, "r")
    if not file then return nil end
    local out, controller_name = {}, nil
    for line in file:lines() do
        local c, _, robot, test, metric, value = line:match("^([^,]*),([^,]*),([^,]*),([^,]*),([^,]*),([^,]*)$")
        if c and c ~= "controller" then
            controller_name = c
            out[robot .. "|" .. test .. "|" .. metric] = tonumber(value)
        end
    end
    file:close()
    return out, controller_name
end

--- The change of every number against results/baseline_<condition>.csv (negative = lower).
local function compare(results)
    local path = RESULTS_DIR .. "baseline_" .. CONDITION .. ".csv"
    local base, base_controller = read_csv(path)
    if not base then
        print("no " .. path .. ": copy latest_" .. CONDITION .. ".csv there to compare later runs with this one")
        return
    end
    print(string.format("\n== change vs %s (controller %s) · new - baseline ==", path, base_controller))
    for _, test_name in ipairs(TEST_NAMES) do
        local test = TESTS[test_name]
        local parts = { string.format("%-14s", test_name) }
        for _, c in ipairs(test.columns) do parts[#parts + 1] = string.format("%9s", c[2]) end
        print(table.concat(parts, " "))
        for _, robot in ipairs(ROBOTS) do
            parts = { string.format("%-14s", robot) }
            for _, c in ipairs(test.columns) do
                local v = results[test_name][robot][c[1]]
                local new = v and v * (c[4] or 1)
                local old = base[robot .. "|" .. test_name .. "|" .. c[2]]
                local d
                if new == nil and old == nil then d = "-"
                elseif new == nil then d = "lost"
                elseif old == nil then d = "new"
                else d = math.abs(new - old) < 0.005 and "0" or string.format("%+.2f", new - old) end
                parts[#parts + 1] = string.format("%9s", d)
            end
            print(table.concat(parts, " "))
        end
    end
end

-- ── the run ────────────────────────────────────────────────────────────────

local jobs, job_i, job, ctx = {}, 0, nil, nil
local phase, ticks, t0, samples
local results, restore, finished = {}, {}, false
local current_robot

for _, test_name in ipairs(TEST_NAMES) do
    results[test_name] = {}
    for _, robot in ipairs(ROBOTS) do jobs[#jobs + 1] = { robot = robot, test = test_name } end
end

local function remember_settings()
    local robot = read_values("sim robot")
    local pairs_ = {}
    for name, value in pairs(robot) do pairs_[#pairs_ + 1] = name .. " " .. string.format("%.17g", value) end
    table.sort(pairs_)
    restore[#restore + 1] = "sim robot set " .. table.concat(pairs_, " ")
    for _, group in ipairs({ "vision", "radio" }) do
        for name, value in pairs(read_values("sim " .. group)) do
            if name == "position_noise" or name == "angle_noise" or name == "delay" then
                restore[#restore + 1] = string.format("sim %s %s %.17g", group, name, value)
            end
        end
    end
    local motion = read_values("motion")
    restore[#restore + 1] = string.format("motion all ramp_accel %.17g", motion.ramp_accel or 2.0)
end

--- What the last run was, as `key=value` lines, for tools that log runs (track.py):
--- the controller and its parameters, the condition's simulator settings, the robots,
--- the tests and which metric ranks each, and the CSV with the results.
local function write_meta(path, csv_path)
    local file = io.open(path, "w")
    if not file then print("could not write " .. path) return end
    local lines = {
        "csv=" .. csv_path, "controller=" .. CONTROLLER, "condition=" .. CONDITION,
        "robots=" .. table.concat(ROBOTS, ","), "tests=" .. table.concat(TEST_NAMES, ","),
    }
    local names = {}
    for k in pairs(P) do names[#names + 1] = k end
    table.sort(names)
    for _, k in ipairs(names) do lines[#lines + 1] = string.format("controller.%s=%.6g", k, P[k]) end
    for _, l in ipairs(CONDITION_LINES) do
        local group, name, value = l:match("^sim (%w+) ([%w_]+) (%S+)$")
        if group then lines[#lines + 1] = string.format("sim.%s.%s=%s", group, name, value) end
    end
    for _, t in ipairs(TEST_NAMES) do
        for _, c in ipairs(TESTS[t].columns) do
            if c[1] == TESTS[t].primary then lines[#lines + 1] = string.format("primary.%s=%s", t, c[2]) end
        end
    end
    file:write(table.concat(lines, "\n") .. "\n")
    file:close()
end

local function finish_all()
    for _, line in ipairs(restore) do sim.console(line) end
    print(string.format("\nSUMMARY controller=%s condition=%s (lower = better)", CONTROLLER, CONDITION))
    local parts = { string.format("%-14s", "robot") }
    for _, t in ipairs(TEST_NAMES) do parts[#parts + 1] = string.format("%18s", t .. ":" .. TESTS[t].primary) end
    print(table.concat(parts, " "))
    for _, robot in ipairs(ROBOTS) do
        parts = { string.format("%-14s", robot) }
        for _, t in ipairs(TEST_NAMES) do
            local col
            for _, c in ipairs(TESTS[t].columns) do if c[1] == TESTS[t].primary then col = c end end
            parts[#parts + 1] = string.format("%18s", cell(results[t][robot], col))
        end
        print(table.concat(parts, " "))
    end
    -- SCORE: the sum of every robot's test costs (see each test's `cost`). Lower is better.
    local total = 0
    local per_robot = {}
    for _, robot in ipairs(ROBOTS) do
        local sum = 0
        for _, t in ipairs(TEST_NAMES) do sum = sum + results[t][robot].cost end
        per_robot[#per_robot + 1] = string.format("%s=%.2f", robot, sum)
        total = total + sum
    end
    print(string.format("SCORE condition=%s total=%.2f  %s", CONDITION, total, table.concat(per_robot, "  ")))
    write_csv(RESULTS_DIR .. "latest_" .. CONDITION .. ".csv", results)
    write_meta(RESULTS_DIR .. "latest.meta", RESULTS_DIR .. "latest_" .. CONDITION .. ".csv")
    compare(results)
    finished = true
    sim.finish(true)
end

local function start_job(i)
    job_i, job = i, jobs[i]
    if not job then return finish_all() end
    if job.robot ~= current_robot then
        console("sim robot load " .. job.robot)
        current_robot = job.robot
    end
    if i == 1 or jobs[i - 1].test ~= job.test then print_header(job.test) end
    ctx = { test = TESTS[job.test] }
    phase, ticks, samples = "hold", 0, {}
end

local TRACE_DIR = os.getenv("CB_TRACE") -- a folder: every test's true trajectory is written there as CSV

local function end_job()
    if TRACE_DIR then
        local f = io.open(string.format("%s/%s_%s.csv", TRACE_DIR, job.test, job.robot), "w")
        if f then
            -- x, y, theta, vx, vy: the robot's true state; goal_*: the pose the controller was
            -- told to reach on that tick; ball_*: the ball as vision sees it. Empty = unknown.
            f:write("t,x,y,theta,vx,vy,goal_x,goal_y,goal_theta,ball_x,ball_y\n")
            local function v(x) return x and string.format("%.5f", x) or "" end
            for _, s in ipairs(samples) do
                f:write(string.format("%.4f,%.5f,%.5f,%.5f,%.4f,%.4f,%s,%s,%s,%s,%s\n", s.t, s.x, s.y, s.th,
                    s.vx, s.vy, v(s.gx), v(s.gy), v(s.gth), v(s.bx), v(s.by)))
            end
            f:close()
        end
    end
    local m = ctx.test.metrics(ctx, samples)
    m.cost = ctx.test.cost(m)
    results[job.test][job.robot] = m
    print_row(job.robot, job.test, m)
    start_job(job_i + 1)
end

function process()
    if finished then return end
    if not job then
        if not sim or sim.console("sim status")[1]:find("^error") then
            print("controller_bench: needs the built-in simulator (headless, or SIM on)")
            finished = true
            if sim then sim.finish(false) end
            return
        end
        remember_settings()
        for _, line in ipairs(CONDITION_LINES) do console(line) end
        console("motion all ramp_accel 0") -- the controller owns the acceleration
        -- Only blue 0 on the field: the others are removed, and put back at the end.
        local removed = 0
        for team, robots in pairs({ [0] = get_blue_team_state(), [1] = get_yellow_team_state() }) do
            for _, r in ipairs(robots) do
                if r.active and not (r.id == ID and team == TEAM) then
                    local color = team == 0 and "blue" or "yellow"
                    console(string.format("sim remove %s %d", color, r.id))
                    table.insert(restore, 1, string.format("sim add %s %d %.4f %.4f %.4f", color, r.id, r.x, r.y, r.orientation))
                    removed = removed + 1
                end
            end
        end
        if removed > 0 then print(string.format("controller_bench: %d other robot(s) removed for the run", removed)) end
        print(string.format("controller_bench: controller=%s v_max=%.2f a_max=%.2f omega_max=%.1f alpha_max=%.1f",
            CONTROLLER, P.v_max, P.a_max, P.omega_max, P.alpha_max))
        return start_job(1)
    end

    if phase == "hold" then
        grsim.teleport_robot(ID, TEAM, 0, 0, 0)
        local b = ctx.test.ball or BALL_PARK
        grsim.teleport_ball(b.x, b.y)
        ticks = ticks + 1
        if ticks >= HOLD_TICKS then
            phase, t0 = "run", sim.time()
            controller:reset()
            ctx.test.init(ctx)
        end
        return
    end

    local t = sim.time() - t0
    local truth = sim.truth(ID, TEAM)
    if truth then
        samples[#samples + 1] = { t = t, x = truth.x, y = truth.y, th = truth.orientation,
            vx = truth.vel_x, vy = truth.vel_y }
    end
    local goal = t < ctx.test.duration and ctx.test.step(ctx, t, truth) or nil
    if not goal then
        send_velocity(ID, TEAM, 0, 0, 0)
        return end_job()
    end
    local last = samples[#samples]
    if last then
        -- The pose the controller is told to reach this tick, and the ball as vision sees it.
        last.gx, last.gy, last.gth = goal.x, goal.y, goal.theta
        local b = get_ball_state()
        if b then last.bx, last.by = b.x, b.y end
    end
    draw_point(goal.x, goal.y, true, { 0.65, 0.55, 0.98 })
    controller:step(ID, TEAM, goal)
end
