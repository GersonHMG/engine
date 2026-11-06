-- motion_control/run.lua — headless benchmark of robot motion control.
--
-- Runs every case in cases.lua with one controller, under each condition (ideal, and
-- with vision noise + delays), measures the robot's TRUE motion (sim.truth: no noise,
-- no delay) and prints a table, a SUMMARY per condition and, if results/baseline.csv
-- exists, the change against the baseline.
--
--   ./target/debug/engine.exe --headless --max-time 600 lua/tests/motion_control/run.lua
--
-- To watch it: open the engine, turn SIM on, load this file and press play. The field
-- shows each case's target, the path from the start and the robot's trail; the results
-- print in the Lua console and the script pauses at the end (reload to run it again).
-- Set CASES below to watch only some cases.
--
-- Environment variables (the GUI reads them too, from where the engine was started):
--   MC_CONTROLLER   controllers/<name>.lua to test (default: baseline)
--   MC_CONDITIONS   comma list of conditions to run (default: ideal,realistic)
--   MC_CASES        comma list of case names to run (default: all)
--
-- Every run writes results/latest.csv. Copy it to results/baseline.csv to make it the
-- reference the next runs are compared with. See README.md.
package.path = "lua/?.lua;" .. package.path

-- Cases to run, e.g. "fwd_4m,stop_moving" (nil = all; MC_CASES overrides this).
local CASES = nil

local cases = require("tests.motion_control.cases")
local ALL_CASES = #cases
do
    local wanted = os.getenv("MC_CASES") or CASES
    if wanted then
        local keep, known = {}, {}
        for _, case in ipairs(cases) do known[case.name] = case end
        for name in wanted:gmatch("[^,%s]+") do
            assert(known[name], "unknown case '" .. name .. "' (see cases.lua)")
            keep[#keep + 1] = known[name]
        end
        cases = keep
    end
end

local CONTROLLER = os.getenv("MC_CONTROLLER") or "baseline"
local controller = require("tests.motion_control.controllers." .. CONTROLLER)

local RESULTS_DIR = "lua/tests/motion_control/results/"

local ROBOT_ID, TEAM = 0, 0
local SETTLE_TICKS = 30      -- ticks the robot is held at the start before each case
local POS_TOL = 0.03         -- m, "on the point"
local ANG_TOL = 0.05         -- rad, "facing the heading"
local FINAL_WINDOW = 0.25    -- s, final errors are averaged over the last FINAL_WINDOW
local TRACK_SKIP = 1.0       -- s, tracking errors are measured after this (catch-up time)
local BALL_PARK = { x = 4.2, y = 2.7 } -- ball out of the way (the path planner avoids it)

-- Conditions: simulator console lines applied before the condition's cases.
-- "realistic" values are guesses until measured on the real robots.
local CONDITIONS = {
    ideal = {
        "sim vision position_noise 0", "sim vision angle_noise 0",
        "sim vision delay 0", "sim radio delay 0",
    },
    realistic = {
        "sim vision position_noise 0.005", "sim vision angle_noise 0.01",
        "sim vision delay 0.033", "sim radio delay 0.033",
    },
}
local CONDITION_ORDER = {}
for name in (os.getenv("MC_CONDITIONS") or "ideal,realistic"):gmatch("[^,%s]+") do
    assert(CONDITIONS[name], "unknown condition '" .. name .. "' (ideal, realistic)")
    CONDITION_ORDER[#CONDITION_ORDER + 1] = name
end

-- ── helpers ────────────────────────────────────────────────────────────────

local function wrap(a)
    while a > math.pi do a = a - 2 * math.pi end
    while a <= -math.pi do a = a + 2 * math.pi end
    return a
end

local function dist(a, b) return math.sqrt((a.x - b.x) ^ 2 + (a.y - b.y) ^ 2) end

--- Shortest time to cover `d` from rest to rest with speed limit v and acceleration a.
local function trapezoid(d, v, a)
    if d <= 0 then return 0 end
    if d >= v * v / a then return d / v + v / a end
    return 2 * math.sqrt(d / a)
end

local function run_console(line)
    local out = sim.console(line)
    for _, l in ipairs(out) do
        if l:lower():find("error") or l:find("unknown") then
            error("console `" .. line .. "`: " .. l)
        end
    end
    return out
end

local function read_profile()
    local p = {}
    for _, line in ipairs(run_console("sim robot")) do
        local name, value = line:match("^%s*([%w_]+)%s*=%s*([-+%d%.eE]+)")
        if name then p[name] = tonumber(value) end
    end
    assert(p.max_speed and p.max_accel and p.max_omega and p.max_alpha, "could not read the robot profile")
    return p
end

local function static_target(case, t)
    if case.switch and t >= case.switch.at then return case.switch.target end
    if type(case.target) == "function" then return case.target(t) end
    return case.target
end

local function final_target(case)
    return case.switch and case.switch.target or case.target
end

local function start_point(case) return { x = case.start[1], y = case.start[2] } end

local function heading_of(case) return case.heading or case.start[3] end

-- ── per-case timing ───────────────────────────────────────────────────────

local profile

local function optimal_time(case)
    if case.kind ~= "point" or case.switch then return nil end
    local d = dist(start_point(case), case.target)
    local turn = math.abs(wrap(heading_of(case) - case.start[3]))
    return math.max(trapezoid(d, profile.max_speed, profile.max_accel),
        trapezoid(turn, profile.max_omega, profile.max_alpha))
end

local function duration_of(case)
    if case.kind == "track" then return case.duration end
    local t_ref = case.switch and case.switch.at or 0
    local d = dist(start_point(case), final_target(case))
    local turn = math.abs(wrap(heading_of(case) - case.start[3]))
    local rough = math.max(trapezoid(d, profile.max_speed, profile.max_accel),
        trapezoid(turn, profile.max_omega, profile.max_alpha))
    return t_ref + math.min(12, math.max(3, 2 * rough + 2))
end

-- ── metrics ────────────────────────────────────────────────────────────────

local function peaks(samples)
    local vmax, amax = 0, 0
    for i, s in ipairs(samples) do
        vmax = math.max(vmax, math.sqrt(s.vx ^ 2 + s.vy ^ 2))
        local p = samples[i - 1]
        if p and s.t > p.t then
            amax = math.max(amax, math.sqrt((s.vx - p.vx) ^ 2 + (s.vy - p.vy) ^ 2) / (s.t - p.t))
        end
    end
    return vmax, amax
end

local function point_metrics(case, samples, duration)
    local T, H = final_target(case), heading_of(case)
    local t_ref = case.switch and case.switch.at or 0
    local m = { duration = duration }

    -- Leg: from where the robot was when it got the final target.
    local leg_start, leg_heading = start_point(case), case.start[3]
    for _, s in ipairs(samples) do
        if s.t >= t_ref then leg_start, leg_heading = { x = s.x, y = s.y }, s.th break end
    end

    -- Settling: the earliest time after which it stays on the point, facing the heading.
    local settled_at = nil
    for i = #samples, 1, -1 do
        local s = samples[i]
        if s.t < t_ref then break end
        local ok = dist(s, T) <= POS_TOL and math.abs(wrap(s.th - H)) <= ANG_TOL
        if not ok then break end
        settled_at = s.t
    end
    m.t_settle = settled_at and (settled_at - t_ref) or nil

    m.t_opt = optimal_time(case)
    if m.t_opt and m.t_settle and m.t_opt > 0 then m.ratio = m.t_settle / m.t_opt end

    -- Overshoot past the target along the leg, and past the heading.
    local d = dist(leg_start, T)
    local ux, uy = 0, 0
    if d > 0.01 then ux, uy = (T.x - leg_start.x) / d, (T.y - leg_start.y) / d end
    local os, dev = 0, 0
    -- Headings unwrapped (continuous), so a 180° turn in either direction is measured right.
    local unwrapped, prev = {}, nil
    for _, s in ipairs(samples) do
        if s.t >= t_ref then
            if d > 0.01 then
                os = math.max(os, (s.x - T.x) * ux + (s.y - T.y) * uy)
                dev = math.max(dev, math.abs((s.x - leg_start.x) * uy - (s.y - leg_start.y) * ux))
            end
            local u = prev and prev + wrap(s.th - prev) or leg_heading + wrap(s.th - leg_heading)
            unwrapped[#unwrapped + 1], prev = u, u
        end
    end
    m.overshoot = d > 0.01 and os or nil
    m.path_dev = (d > 0.01 and not case.switch) and dev or nil
    if #unwrapped > 0 then
        -- The target heading nearest to where the robot ended, and the way it turned to get there.
        local u_end = unwrapped[#unwrapped]
        local u_target = u_end + wrap(H - u_end)
        local turned = u_target - leg_heading
        if math.abs(turned) > 0.01 then
            local sign = turned > 0 and 1 or -1
            local ang_os = 0
            for _, u in ipairs(unwrapped) do ang_os = math.max(ang_os, sign * (u - u_target)) end
            m.ang_overshoot = ang_os
        end
    end

    -- Final errors: mean over the last FINAL_WINDOW.
    local n, pe, ae = 0, 0, 0
    for _, s in ipairs(samples) do
        if s.t >= duration - FINAL_WINDOW then
            n = n + 1
            pe = pe + dist(s, T)
            ae = ae + math.abs(wrap(s.th - H))
        end
    end
    m.final_pos_err = n > 0 and pe / n or nil
    m.final_ang_err = n > 0 and ae / n or nil
    m.peak_speed, m.peak_accel = peaks(samples)
    return m
end

local function track_metrics(case, samples, duration)
    local m = { duration = duration }
    local n, sq, worst = 0, 0, 0
    for _, s in ipairs(samples) do
        if s.t >= TRACK_SKIP then
            local e = dist(s, case.target(s.t))
            n, sq, worst = n + 1, sq + e * e, math.max(worst, e)
        end
    end
    m.track_rms = n > 0 and math.sqrt(sq / n) or nil
    m.track_max = n > 0 and worst or nil
    m.peak_speed, m.peak_accel = peaks(samples)
    return m
end

-- ── results: printing, CSV, summary, comparison ────────────────────────────

local COLUMNS = { "t_settle", "t_opt", "ratio", "overshoot", "ang_overshoot", "path_dev",
    "final_pos_err", "final_ang_err", "peak_speed", "peak_accel", "track_rms", "track_max", "duration" }

local function f(v, fmt, scale)
    if v == nil then return "-" end
    return string.format(fmt, v * (scale or 1))
end

local function print_header(condition)
    print(string.format("\n== %s · controller %s ==", condition, CONTROLLER))
    print(string.format("%-13s %8s %6s %5s %6s %6s %6s %7s %6s %5s %6s %8s",
        "case", "settle_s", "opt_s", "ratio", "os_mm", "os_deg", "dev_mm",
        "err_mm", "err_deg", "v_max", "a_max", "track_mm"))
end

local function print_row(name, m)
    local settle = m.t_settle and string.format("%.2f", m.t_settle) or "NO"
    print(string.format("%-13s %8s %6s %5s %6s %6s %6s %7s %6s %5s %6s %8s",
        name, settle, f(m.t_opt, "%.2f"), f(m.ratio, "%.2f"),
        f(m.overshoot, "%.0f", 1000), f(m.ang_overshoot, "%.1f", 180 / math.pi), f(m.path_dev, "%.0f", 1000),
        f(m.final_pos_err, "%.1f", 1000), f(m.final_ang_err, "%.2f", 180 / math.pi),
        f(m.peak_speed, "%.2f"), f(m.peak_accel, "%.1f"),
        m.track_rms and string.format("%.0f/%.0f", m.track_rms * 1000, m.track_max * 1000) or "-"))
end

--- Summary of one condition's rows: { name = metrics } with `kind`.
--- `only`: count only the cases present there (to compare a partial run with the baseline).
local function summarize(rows, only)
    local s = { points = 0, settled = 0, score = 0 }
    local sums = { ratio = { 0, 0 }, overshoot = { 0, 0 }, final_pos_err = { 0, 0 }, track_rms = { 0, 0 } }
    for _, case in ipairs(cases) do
        local m = rows[case.name]
        if m and (not only or only[case.name]) then
            if case.kind == "point" then
                s.points = s.points + 1
                local t_ref = case.switch and case.switch.at or 0
                if m.t_settle then
                    s.settled = s.settled + 1
                    s.score = s.score + m.t_settle
                else
                    s.score = s.score + (m.duration - t_ref)
                end
            end
            for k, acc in pairs(sums) do
                if m[k] then acc[1], acc[2] = acc[1] + m[k], acc[2] + 1 end
            end
        end
    end
    for k, acc in pairs(sums) do s[k] = acc[2] > 0 and acc[1] / acc[2] or nil end
    return s
end

local function summary_line(condition, s)
    return string.format("SUMMARY %-9s settled=%d/%d SCORE=%.2f s  mean_ratio=%s  mean_overshoot=%s mm  mean_final_err=%s mm  track_rms=%s mm",
        condition, s.settled, s.points, s.score, f(s.ratio, "%.2f"), f(s.overshoot, "%.1f", 1000),
        f(s.final_pos_err, "%.1f", 1000), f(s.track_rms, "%.1f", 1000))
end

local function write_csv(path, all)
    local file, err = io.open(path, "w")
    if not file then
        print("could not write " .. path .. ": " .. tostring(err))
        return false
    end
    file:write("condition,case,controller," .. table.concat(COLUMNS, ",") .. "\n")
    for _, condition in ipairs(CONDITION_ORDER) do
        for _, case in ipairs(cases) do
            local m = all[condition][case.name]
            if m then
                local cells = { condition, case.name, CONTROLLER }
                for _, c in ipairs(COLUMNS) do
                    cells[#cells + 1] = m[c] and string.format("%.6g", m[c]) or ""
                end
                file:write(table.concat(cells, ",") .. "\n")
            end
        end
    end
    file:close()
    return true
end

--- { condition = { case = metrics } } from a CSV written by write_csv, or nil.
local function read_csv(path)
    local file = io.open(path, "r")
    if not file then return nil end
    local header, result = nil, {}
    for line in file:lines() do
        local cells = {}
        for cell in (line .. ","):gmatch("([^,]*),") do cells[#cells + 1] = cell end
        if not header then
            header = cells
        else
            local m = {}
            for i, name in ipairs(header) do
                if i > 3 and cells[i] ~= "" then m[name] = tonumber(cells[i]) end
            end
            result[cells[1]] = result[cells[1]] or {}
            result[cells[1]][cells[2]] = m
            result.controller = cells[3]
        end
    end
    file:close()
    return result
end

local function delta(new, old, fmt, scale)
    if new == nil and old == nil then return "-" end
    if new == nil then return "NO" end
    if old == nil then return "new" end
    return string.format(fmt, (new - old) * (scale or 1))
end

local function print_comparison(all, base)
    print(string.format("\n== change vs results/baseline.csv (controller %s) · negative = better ==",
        base.controller or "?"))
    for _, condition in ipairs(CONDITION_ORDER) do
        local b = base[condition]
        if not b then
            print(string.format("%s: not in the baseline", condition))
        else
            print(string.format("-- %s", condition))
            print(string.format("%-13s %9s %8s %8s %9s", "case", "settle_s", "os_mm", "err_mm", "track_mm"))
            for _, case in ipairs(cases) do
                local m, o = all[condition][case.name], b[case.name]
                if m and o then
                    print(string.format("%-13s %9s %8s %8s %9s", case.name,
                        delta(m.t_settle, o.t_settle, "%+.2f"),
                        delta(m.overshoot, o.overshoot, "%+.0f", 1000),
                        delta(m.final_pos_err, o.final_pos_err, "%+.1f", 1000),
                        delta(m.track_rms, o.track_rms, "%+.0f", 1000)))
                end
            end
            local sn, so = summarize(all[condition]), summarize(b, all[condition])
            print(string.format("DELTA %-9s settled %+d  SCORE %+.2f s (%.2f -> %.2f)  overshoot %s mm  final_err %s mm  track_rms %s mm",
                condition, sn.settled - so.settled, sn.score - so.score, so.score, sn.score,
                delta(sn.overshoot, so.overshoot, "%+.1f", 1000),
                delta(sn.final_pos_err, so.final_pos_err, "%+.1f", 1000),
                delta(sn.track_rms, so.track_rms, "%+.1f", 1000)))
        end
    end
end

-- ── the run: one job per (condition, case) ─────────────────────────────────

local jobs, job_i, finished = {}, 0, false
local job, phase, ticks, t0, samples
local results = {}

-- The simulator's vision/radio settings before the run, put back at the end (GUI).
local restore = {}

local function remember_settings()
    for _, group in ipairs({ "vision", "radio" }) do
        for _, line in ipairs(run_console("sim " .. group)) do
            local name, value = line:match("^%s*([%w_]+)%s*=%s*([-+%d%.eE]+)")
            if name then restore[#restore + 1] = string.format("sim %s %s %s", group, name, value) end
        end
    end
end

local function finish_all()
    for _, line in ipairs(restore) do sim.console(line) end
    for _, condition in ipairs(CONDITION_ORDER) do
        print(summary_line(condition, summarize(results[condition])))
    end
    if #cases == ALL_CASES then
        write_csv(RESULTS_DIR .. "latest.csv", results)
        print("wrote " .. RESULTS_DIR .. "latest.csv")
    else
        print("only some cases ran: " .. RESULTS_DIR .. "latest.csv not written")
    end
    local base = read_csv(RESULTS_DIR .. "baseline.csv")
    if base then
        print_comparison(results, base)
    else
        print("no " .. RESULTS_DIR .. "baseline.csv yet: copy latest.csv there to compare later runs with this one")
    end
    finished = true
    sim.finish(true)
end

local function start_job(i)
    job_i = i
    job = jobs[i]
    if not job then return finish_all() end
    if job.first_of_condition then
        for _, line in ipairs(CONDITIONS[job.condition]) do run_console(line) end
        results[job.condition] = {}
        print_header(job.condition)
    end
    job.duration = duration_of(job.case)
    phase, ticks, samples = "settle", 0, {}
end

local function end_job()
    local case = job.case
    local m = case.kind == "track" and track_metrics(case, samples, job.duration)
        or point_metrics(case, samples, job.duration)
    results[job.condition][case.name] = m
    print_row(case.name, m)
    start_job(job_i + 1)
end

--- What the field shows: the case, its target, the leg from the start, the robot's trail.
local function draw(case, t)
    local target = static_target(case, t or 0)
    draw_text(-4.4, 3.2, string.format("%s · %s%s", job.condition, case.name,
        t and string.format("  %.1f / %.1f s", t, job.duration) or ""), { 1, 1, 1 })
    if case.kind == "point" then
        local from = start_point(case)
        if case.switch and t and t >= case.switch.at then
            draw_point(case.target.x, case.target.y, true, { 0.5, 0.5, 0.5 })
        end
        draw_line({ from, target }, { 0.5, 0.5, 0.5 })
    end
    draw_point(target.x, target.y, true, { 0.65, 0.55, 0.98 })
    local h = heading_of(case)
    draw_line({ target, { x = target.x + 0.25 * math.cos(h), y = target.y + 0.25 * math.sin(h) } }, { 0.65, 0.55, 0.98 })
    if #samples > 1 then
        local trail = {}
        for i = 1, #samples, 3 do trail[#trail + 1] = { x = samples[i].x, y = samples[i].y } end
        trail[#trail + 1] = { x = samples[#samples].x, y = samples[#samples].y }
        draw_line(trail, { 0.24, 0.86, 0.52 })
    end
end

function process()
    if finished then return end
    if not jobs[1] then
        -- First tick: check the simulator, read the robot's limits and list the jobs.
        if not sim or sim.console("sim status")[1]:find("^error") then
            print("motion_control: needs the built-in simulator: turn SIM on and reload this script")
            finished = true
            if sim then sim.finish(false) end
            return
        end
        local others = #get_blue_team_state() + #get_yellow_team_state() - 1
        if others > 0 then
            print(string.format("motion_control: %d other robot(s) on the field: the path planner avoids them, so results can differ from the baseline", others))
        end
        remember_settings()
        profile = read_profile()
        print(string.format("motion_control: controller=%s  robot max_speed=%.2f max_accel=%.2f max_omega=%.2f max_alpha=%.2f",
            CONTROLLER, profile.max_speed, profile.max_accel, profile.max_omega, profile.max_alpha))
        for _, condition in ipairs(CONDITION_ORDER) do
            for k, case in ipairs(cases) do
                jobs[#jobs + 1] = { condition = condition, case = case, first_of_condition = k == 1 }
            end
        end
        grsim.teleport_ball(BALL_PARK.x, BALL_PARK.y)
        return start_job(1)
    end
    if not job then return end
    local case = job.case

    if phase == "settle" then
        -- Hold the robot at rest at the start, with no commands, so commands and
        -- vision frames still in flight from the previous case are flushed.
        grsim.teleport_robot(ROBOT_ID, TEAM, case.start[1], case.start[2], case.start[3])
        draw(case, nil)
        ticks = ticks + 1
        if ticks >= SETTLE_TICKS then
            phase, t0 = "run", sim.time()
        end
        return
    end

    local t = sim.time() - t0
    local truth = sim.truth(ROBOT_ID, TEAM)
    if truth then
        samples[#samples + 1] = { t = t, x = truth.x, y = truth.y, th = truth.orientation,
            vx = truth.vel_x, vy = truth.vel_y }
    end
    if t >= job.duration then return end_job() end

    draw(case, t)
    controller(ROBOT_ID, TEAM, static_target(case, t), heading_of(case))
end
