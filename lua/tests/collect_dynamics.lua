-- collect_dynamics: drives one robot with excitation commands and logs every tick, to learn
-- the robot's dynamics (nn_sim). Works with grSim (GUI, SIM off) and the built-in simulator.
--
--   GUI + grSim:  load lua/tests/collect_mix.lua or lua/tests/collect_waypoint.lua, press play
--   Dry run:      ./target/release/engine --headless --max-time 120 lua/tests/collect_mix.lua
--
-- Each episode: teleport the robot near the center at rest (phase 0, "settle"), then drive it
-- with a random command signal (phase 1, "excite") until EPISODE_S or until it leaves the
-- safety box. Every tick writes one CSV row: the robot state seen this tick and the command
-- sent this tick (vt forward, vn left, w counter-clockwise, robot frame, as send_velocity).
--
-- Output: logs/dyn_<mode>_<date>_seed<seed>.csv and a .json with the settings.
-- Settings: the CONFIG table below; a wrapper script may override any of them through the
-- global COLLECT before requiring this file.

local CONFIG = {
    mode = "mix",              -- "mix" (training signals) or "waypoint" (AI-like, for testing)
    robot_id = 0,
    team = 0,                  -- 0 blue, 1 yellow
    seed = 1,                  -- change it for every recording
    duration_min = 15,         -- minutes of excitation (phase 1) to record, then stop
    max_speed = 3.5,           -- m/s, nominal robot limit; step commands go up to overshoot x this
    max_omega = 10.0,          -- rad/s, nominal limit
    overshoot = 1.3,           -- step commands exceed the limits by this factor
    episode_s = 6.0,           -- s, longest mix episode
    waypoint_episode_s = 40.0, -- s, longest waypoint episode
    safe_x = 3.6,              -- m, episode ends when |x| > safe_x (walls at 4.8)
    safe_y = 2.4,              -- m, episode ends when |y| > safe_y (walls at 3.3)
    start_x = 1.0,             -- m, episode start: uniform in [-start_x, start_x]
    start_y = 0.7,             -- m
    settle_s = 0.5,            -- s at rest after each teleport (logged as phase 0)
    park_y = 3.1,              -- m, other robots are parked along this line, out of the way
    ball = { x = 4.3, y = -3.1 },
    out_dir = "logs",
}
for k, v in pairs(rawget(_G, "COLLECT") or {}) do
    CONFIG[k] = v
end

local TICK = 1 / 60
local ID, TEAM = CONFIG.robot_id, CONFIG.team
local headless = rawget(_G, "sim") ~= nil
local SEG = { step = 0, multisine = 1, noise = 2, chirp = 3, idle = 4, waypoint = 5 }

assert(wall_time, "wall_time() is missing: rebuild the engine (cargo build --release) with the nn_sim changes")
math.randomseed(CONFIG.seed)

local function uniform(lo, hi) return lo + (hi - lo) * math.random() end
local function chance(p) return math.random() < p end
local function log_uniform(lo, hi) return math.exp(uniform(math.log(lo), math.log(hi))) end
local function wrap(a)
    while a > math.pi do a = a - 2 * math.pi end
    while a <= -math.pi do a = a + 2 * math.pi end
    return a
end
local function ticks(seconds) return math.max(1, math.floor(seconds / TICK + 0.5)) end

-- ─── Command signals: each returns { seg, left, next(robot) -> vt, vn, w } ─────────────

local function step_signal()
    local speed = chance(0.15) and 0 or uniform(0, CONFIG.overshoot * CONFIG.max_speed)
    local dir = uniform(-math.pi, math.pi)
    local w = chance(0.25) and 0 or uniform(-CONFIG.overshoot, CONFIG.overshoot) * CONFIG.max_omega
    local vt, vn = speed * math.cos(dir), speed * math.sin(dir)
    return { seg = SEG.step, left = ticks(uniform(0.05, 1.5)), next = function() return vt, vn, w end }
end

local function idle_signal()
    return { seg = SEG.idle, left = ticks(uniform(0.2, 1.0)), next = function() return 0, 0, 0 end }
end

-- One smooth channel in [-1, 1]: a sum of sinusoids, a chirp or low-pass filtered noise.
local function wave(kind, duration)
    if kind == "multisine" then
        local parts, total = {}, 0
        for i = 1, 1 + math.floor(math.random() * 3) do
            parts[i] = { w = math.random(), f = log_uniform(0.1, 3.0), ph = uniform(-math.pi, math.pi) }
            total = total + parts[i].w
        end
        return function(t)
            local s = 0
            for _, p in ipairs(parts) do s = s + p.w / math.max(total, 1e-9) * math.sin(2 * math.pi * p.f * t + p.ph) end
            return s
        end
    elseif kind == "chirp" then
        local f0, f1, ph = log_uniform(0.1, 3.0), log_uniform(0.1, 3.0), uniform(-math.pi, math.pi)
        local k = (f1 - f0) / duration
        return function(t) return math.sin(2 * math.pi * (f0 * t + 0.5 * k * t * t) + ph) end
    else
        local alpha = 1 - math.exp(-2 * math.pi * log_uniform(0.3, 3.0) * TICK)
        local gain = math.sqrt(2 / alpha)
        local state = uniform(-1, 1) / gain
        return function()
            state = state + alpha * (uniform(-1, 1) - state)
            return math.max(-1, math.min(1, state * gain))
        end
    end
end

local function smooth_signal(kind)
    local duration = uniform(1.0, 3.0)
    local limits = { CONFIG.max_speed, CONFIG.max_speed, CONFIG.overshoot * CONFIG.max_omega }
    local ch = {}
    for i = 1, 3 do
        ch[i] = {
            amp = chance(0.15) and 0 or uniform(0, 1) * limits[i],
            offset = uniform(-0.3, 0.3) * limits[i],
            wave = wave(kind, duration),
        }
    end
    local t = 0
    return {
        seg = SEG[kind],
        left = ticks(duration),
        next = function()
            local v = {}
            for i = 1, 3 do v[i] = ch[i].offset + ch[i].amp * ch[i].wave(t) end
            t = t + TICK
            return v[1], v[2], v[3]
        end,
    }
end

local function next_mix()
    local r = math.random()
    if r < 0.40 then return step_signal()
    elseif r < 0.60 then return smooth_signal("multisine")
    elseif r < 0.75 then return smooth_signal("noise")
    elseif r < 0.85 then return smooth_signal("chirp")
    else return idle_signal() end
end

-- Go-to-point controller like an AI's: slows down on approach, never above the limits.
local function waypoint_signal(robot)
    local margin = 0.5
    local target = { x = uniform(-CONFIG.safe_x + margin, CONFIG.safe_x - margin),
                     y = uniform(-CONFIG.safe_y + margin, CONFIG.safe_y - margin) }
    local dir = math.atan(target.y - robot.y, target.x - robot.x)
    local heading = chance(0.5) and dir or uniform(-math.pi, math.pi)
    local vcap = uniform(0.3, 0.9) * CONFIG.max_speed
    local accel = uniform(0.3, 0.9) * 6.0
    local wcap = uniform(0.2, 0.8) * CONFIG.max_omega
    local sig = { seg = SEG.waypoint, left = ticks(5.0) }
    sig.next = function(r)
        local dx, dy = target.x - r.x, target.y - r.y
        local dist = math.sqrt(dx * dx + dy * dy)
        local herr = wrap(heading - r.orientation)
        if dist < 0.03 and math.abs(herr) < 0.05 then
            sig.left = math.min(sig.left, ticks(uniform(0.0, 0.3)))
        end
        local speed = math.min(vcap, math.sqrt(2 * accel * dist))
        local wx, wy = 0, 0
        if dist > 0.01 then wx, wy = dx / dist * speed, dy / dist * speed end
        local c, s = math.cos(r.orientation), math.sin(r.orientation)
        return c * wx + s * wy, -s * wx + c * wy, math.max(-wcap, math.min(wcap, 4 * herr))
    end
    return sig
end

-- ─── Output ─────────────────────────────────────────────────────────────────────────────

local file, path
local function open_output()
    local name = string.format("dyn_%s_%s_seed%d", CONFIG.mode, os.date("%Y%m%d_%H%M%S"), CONFIG.seed)
    path = CONFIG.out_dir .. "/" .. name
    file = io.open(path .. ".csv", "w")
    if not file then
        os.execute("mkdir " .. CONFIG.out_dir)
        file = io.open(path .. ".csv", "w")
    end
    if not file then
        path = name
        file = assert(io.open(path .. ".csv", "w"), "cannot create the output file")
    end
    file:write("wall_t,tick,episode,seg,phase,active,vt_cmd,vn_cmd,w_cmd,x,y,theta,vx,vy,omega\n")
    local meta = io.open(path .. ".json", "w")
    local keys = {}
    for k in pairs(CONFIG) do keys[#keys + 1] = k end
    table.sort(keys)
    local items = {}
    for _, k in ipairs(keys) do
        local v = CONFIG[k]
        if type(v) == "table" then
            v = string.format('{"x": %s, "y": %s}', v.x, v.y)
        elseif type(v) == "string" then
            v = '"' .. v .. '"'
        else
            v = tostring(v)
        end
        items[#items + 1] = string.format('  "%s": %s', k, v)
    end
    items[#items + 1] = string.format('  "started": "%s"', os.date("%Y-%m-%d %H:%M:%S"))
    items[#items + 1] = string.format('  "source": "%s"', headless and "builtin-simulator-headless" or "engine-gui")
    meta:write("{\n" .. table.concat(items, ",\n") .. "\n}\n")
    meta:close()
    print("[collect] writing " .. path .. ".csv")
end

local tick, rows_since_flush = 0, 0
local function log_row(episode, seg, phase, r, vt, vn, w)
    file:write(string.format("%.6f,%d,%d,%d,%d,%d,%.6g,%.6g,%.6g,%.6f,%.6f,%.6f,%.6f,%.6f,%.6f\n",
        wall_time(), tick, episode, seg, phase, r.active and 1 or 0, vt, vn, w,
        r.x, r.y, r.orientation, r.vel_x, r.vel_y, r.omega))
    rows_since_flush = rows_since_flush + 1
    if rows_since_flush >= 60 then
        file:flush()
        rows_since_flush = 0
    end
end

-- ─── Field setup ────────────────────────────────────────────────────────────────────────

local function park_others()
    local spot = 0
    local function park(list, team)
        for _, r in ipairs(list) do
            if r.active and not (r.id == ID and team == TEAM) and math.abs(r.y) < CONFIG.park_y - 0.2 then
                grsim.teleport_robot(r.id, team, -4.3 + 0.25 * spot, CONFIG.park_y, -math.pi / 2)
                spot = spot + 1
            end
        end
    end
    park(get_blue_team_state(), 0)
    park(get_yellow_team_state(), 1)
    grsim.teleport_ball(CONFIG.ball.x, CONFIG.ball.y)
end

-- ─── Main loop ──────────────────────────────────────────────────────────────────────────

local state = "init"
local episode, settle_left, episode_left, signal = 0, 0, 0, nil
local start_pose
local excite_ticks, target_ticks = 0, ticks(CONFIG.duration_min * 60)
local ended = { time = 0, box = 0, vision = 0 }

local function begin_episode()
    episode = episode + 1
    start_pose = { x = uniform(-CONFIG.start_x, CONFIG.start_x), y = uniform(-CONFIG.start_y, CONFIG.start_y),
                   th = uniform(-math.pi, math.pi) }
    if episode % 20 == 1 then park_others() end
    grsim.teleport_robot(ID, TEAM, start_pose.x, start_pose.y, start_pose.th)
    settle_left = ticks(CONFIG.settle_s)
    state = "settle"
end

local function finish()
    state = "done"
    file:close()
    print(string.format("[collect] done: %d episodes, %.1f min of excitation; ended by time %d, box %d, vision loss %d",
        episode, excite_ticks * TICK / 60, ended.time, ended.box, ended.vision))
    print("[collect] saved " .. path .. ".csv")
    if headless then sim.finish(true) end
end

function process()
    if state == "done" then return end
    tick = tick + 1
    local r = get_robot_state(ID, TEAM)

    if state == "init" then
        if not r.active then
            if tick % 60 == 1 then print(string.format("[collect] waiting for robot %d (team %d) in vision...", ID, TEAM)) end
            return
        end
        open_output()
        print(string.format("[collect] mode %s, seed %d, target %.1f min of excitation", CONFIG.mode, CONFIG.seed, CONFIG.duration_min))
        begin_episode()
        return
    end

    if state == "settle" then
        send_velocity(ID, TEAM, 0, 0, 0)
        log_row(episode, SEG.idle, 0, r, 0, 0, 0)
        settle_left = settle_left - 1
        if settle_left <= 0 then
            local d = math.sqrt((r.x - start_pose.x) ^ 2 + (r.y - start_pose.y) ^ 2)
            if not r.active or d > 0.1 then
                -- The teleport has not shown up in vision yet (or failed): try again.
                begin_episode()
                return
            end
            signal = CONFIG.mode == "waypoint" and waypoint_signal(r) or next_mix()
            episode_left = ticks(CONFIG.mode == "waypoint" and CONFIG.waypoint_episode_s or CONFIG.episode_s)
            state = "excite"
        end
        return
    end

    -- state == "excite"
    local reason
    if not r.active then reason = "vision"
    elseif math.abs(r.x) > CONFIG.safe_x or math.abs(r.y) > CONFIG.safe_y then reason = "box"
    elseif episode_left <= 0 then reason = "time" end
    if reason then
        ended[reason] = ended[reason] + 1
        send_velocity(ID, TEAM, 0, 0, 0)
        if excite_ticks >= target_ticks then return finish() end
        if episode % 10 == 0 then
            print(string.format("[collect] episode %d, %.1f / %.1f min", episode, excite_ticks * TICK / 60, CONFIG.duration_min))
        end
        return begin_episode()
    end

    if signal.left <= 0 then
        signal = CONFIG.mode == "waypoint" and waypoint_signal(r) or next_mix()
    end
    local vt, vn, w = signal.next(r)
    signal.left = signal.left - 1
    send_velocity(ID, TEAM, vt, vn, w)
    log_row(episode, signal.seg, 1, r, vt, vn, w)
    episode_left = episode_left - 1
    excite_ticks = excite_ticks + 1
end
