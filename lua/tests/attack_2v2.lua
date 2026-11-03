-- attack_2v2: how often the blue AI (lua/AI) scores against the run_ai.lua defense.
--
-- Objective: win rate = episodes with a goal / episodes.
--   win   the ball crosses x > 4.5 with |y| < GOAL_HALF_WIDTH within EPISODE_TIME
--   loss  timeout, ball out of the field, or a yellow robot controls the ball (closest
--         robot, within CONTROL_DIST, ball slower than CONTROL_SPEED for CONTROL_TIME)
-- Episodes start from reproducible random setups (own LCG): ball x in [0, 3.5],
-- |y| <= 2.5; blue 0 near the ball, blue 1 anywhere in the attacking half; yellow 0 on
-- its marking spot, yellow 1 on its guard spot; everything at rest.
--
-- Settings (environment variables):
--   SEEDS   tuning (default) | holdout | named      which start set
--   N       episodes (default 100; named: 2)
--   RECORD  path of a JSON file with the trajectories of the first RECORD_N episodes
--   RECORD_N  default 12
--
-- Run from the repository root:
--   SEEDS=holdout ./target/debug/engine --headless --max-time 4000 lua/tests/attack_2v2.lua
package.path = "lua/?.lua;" .. package.path

local ai = require("AI.main")
local defense = require("tests.lib.defense_2v2")
local mark = require("skills.mark")
local goalkeeper = require("skills.goalkeeper")

local EPISODE_TIME = 15.0       -- s
local GOAL_X = 4.5              -- m, the goal line blue attacks
local GOAL_HALF_WIDTH = 0.5     -- m
local FIELD_HALF_X, FIELD_HALF_Y = 4.5, 3.0  -- m
local CONTROL_DIST = 0.16       -- m, robot center to ball for "controls the ball"
local CONTROL_SPEED = 0.5       -- m/s
local CONTROL_TIME = 0.5        -- s
local MIN_GAP = 0.3             -- m between robots at the start
local RECORD_EVERY = 2          -- ticks between recorded frames

local SET = os.getenv("SEEDS") or "tuning"
local N = tonumber(os.getenv("N") or "") or 100
local RECORD = os.getenv("RECORD")
local RECORD_N = tonumber(os.getenv("RECORD_N") or "") or 12
local BASE = ({ tuning = 1, holdout = 2, named = 3 })[SET] or 1

-- ---------------------------------------------------------------------------
-- starts
-- ---------------------------------------------------------------------------

local function lcg(seed)
    local s = seed % 2147483647
    if s <= 0 then s = s + 2147483646 end
    return function(a, b)
        s = (s * 48271) % 2147483647
        return a + (b - a) * (s / 2147483647)
    end
end

local function dist(a, b)
    return math.sqrt((a.x - b.x) ^ 2 + (a.y - b.y) ^ 2)
end

local function facing(from, to)
    return math.atan(to.y - from.y, to.x - from.x)
end

--- Places the defense the way it plays: marker on its spot, keeper on its guard spot.
local function defense_for(start)
    local free = dist(start.b0, start.ball) >= dist(start.b1, start.ball) and start.b0 or start.b1
    start.y0 = mark.spot(free, start.ball)
    start.y1 = goalkeeper.spot(start.ball, defense.GOAL)
end

local function valid(start)
    local robots = { start.b0, start.b1, start.y0, start.y1 }
    for i = 1, #robots do
        if math.abs(robots[i].x) > FIELD_HALF_X - 0.1 or math.abs(robots[i].y) > FIELD_HALF_Y - 0.1 then
            return false
        end
        if dist(robots[i], start.ball) < 0.2 then return false end
        for j = i + 1, #robots do
            if dist(robots[i], robots[j]) < MIN_GAP then return false end
        end
    end
    return true
end

local function random_start(index)
    local rand = lcg(BASE * 100003 + index * 7919)
    for _ = 1, 200 do
        local s = { ball = { x = rand(0.0, 3.5), y = rand(-2.5, 2.5) } }
        local a, d = rand(-math.pi, math.pi), rand(0.5, 1.5)
        s.b0 = { x = s.ball.x + d * math.cos(a), y = s.ball.y + d * math.sin(a) }
        s.b1 = { x = rand(0.0, 3.5), y = rand(-2.5, 2.5) }
        defense_for(s)
        if valid(s) then return s end
    end
    error("no valid start for episode " .. index)
end

-- scenarios/2vs1.toml and scenarios/marked_pass.toml
local NAMED = {
    { ball = { x = -1.128, y = 0.024 }, b0 = { x = -1.553, y = -0.840 }, b1 = { x = -1.268, y = 0.534 },
      y0 = { x = 2.422, y = 0.480 }, y1 = { x = 4.356, y = -0.021 } },
    { ball = { x = 3.919, y = 1.325 }, b0 = { x = 2.497, y = -0.998 }, b1 = { x = 3.842, y = 1.481 },
      y0 = { x = 2.116, y = -0.506 }, y1 = { x = 4.350, y = 0.317 } },
}
if SET == "named" then N = #NAMED end

local function start_for(index)
    if SET == "named" then return NAMED[index] end
    return random_start(index)
end

-- ---------------------------------------------------------------------------
-- episode state
-- ---------------------------------------------------------------------------

local episode, start, t0
local control_since, last_touch, kicks
local results, counts = {}, {}
local recorded, frames, tick = {}, nil, 0

local function begin(index)
    episode = index
    start = start_for(index)
    local s = start
    grsim.teleport_robot(0, 0, s.b0.x, s.b0.y, facing(s.b0, s.ball))
    grsim.teleport_robot(1, 0, s.b1.x, s.b1.y, facing(s.b1, s.ball))
    grsim.teleport_robot(defense.MARKER_ID, defense.YELLOW, s.y0.x, s.y0.y, facing(s.y0, s.ball))
    grsim.teleport_robot(defense.KEEPER_ID, defense.YELLOW, s.y1.x, s.y1.y, facing(s.y1, s.ball))
    grsim.teleport_ball(s.ball.x, s.ball.y)
    ai.reset()
    sim.events()
    t0 = sim.time()
    control_since, last_touch, kicks = nil, nil, {}
    frames = (RECORD and index <= RECORD_N) and {} or nil
    tick = 0
end

local function finish_episode(result, detail)
    local t = sim.time() - t0
    counts[result] = (counts[result] or 0) + 1
    local kick_names = {}
    for _, k in ipairs(kicks) do kick_names[#kick_names + 1] = k.action end
    results[#results + 1] = { index = episode, result = result, time = t }
    print(string.format("%-3d %-9s t=%5.2f s  kicks=[%s]%s", episode, result, t,
        table.concat(kick_names, ","), detail and ("  " .. detail) or ""))
    if frames then
        recorded[#recorded + 1] = { index = episode, result = result, time = t, kicks = kicks, frames = frames }
    end
    if episode >= N then
        return true
    end
    begin(episode + 1)
    return false
end

-- ---------------------------------------------------------------------------
-- output
-- ---------------------------------------------------------------------------

local function json(value)
    local kind = type(value)
    if kind == "number" then
        if value ~= value or value == math.huge or value == -math.huge then return "null" end
        return string.format("%.3f", value):gsub("%.?0+$", "")
    elseif kind == "string" then
        return '"' .. value:gsub('[%c"\\]', "") .. '"'
    elseif kind == "boolean" then
        return tostring(value)
    elseif kind == "table" then
        local parts = {}
        if #value > 0 or next(value) == nil then
            for _, v in ipairs(value) do parts[#parts + 1] = json(v) end
            return "[" .. table.concat(parts, ",") .. "]"
        end
        for k, v in pairs(value) do parts[#parts + 1] = '"' .. k .. '":' .. json(v) end
        table.sort(parts)
        return "{" .. table.concat(parts, ",") .. "}"
    end
    return "null"
end

local function summary()
    local wins = counts.goal or 0
    local n = #results
    local p = wins / n
    -- Wilson 95% interval
    local z = 1.96
    local centre = (p + z * z / (2 * n)) / (1 + z * z / n)
    local half = z * math.sqrt(p * (1 - p) / n + z * z / (4 * n * n)) / (1 + z * z / n)
    local names = {}
    for k in pairs(counts) do names[#names + 1] = k end
    table.sort(names)
    local hist = {}
    for _, k in ipairs(names) do hist[#hist + 1] = k .. "=" .. counts[k] end
    print(string.format("SUMMARY set=%s win=%d/%d (%.1f%%, 95%% CI %.1f-%.1f%%) %s",
        SET, wins, n, 100 * p, 100 * (centre - half), 100 * (centre + half), table.concat(hist, " ")))
    if RECORD then
        local f = assert(io.open(RECORD, "w"))
        f:write(json({ set = SET, win = wins, n = n, episodes = recorded }))
        f:close()
    end
end

-- ---------------------------------------------------------------------------
-- tick
-- ---------------------------------------------------------------------------

local function robots()
    return {
        get_robot_state(0, 0), get_robot_state(1, 0),
        get_robot_state(defense.MARKER_ID, defense.YELLOW), get_robot_state(defense.KEEPER_ID, defense.YELLOW),
    }
end

local function record_frame(ball, rs)
    tick = tick + 1
    if not frames or tick % RECORD_EVERY ~= 1 then return end
    local f = { sim.time() - t0, ball.x, ball.y }
    for _, r in ipairs(rs) do
        f[#f + 1] = r.x; f[#f + 1] = r.y; f[#f + 1] = r.orientation
    end
    local c0, c1 = ai.chosen[0], ai.chosen[1]
    f[#f + 1] = c0 and c0.action or ""
    f[#f + 1] = c1 and c1.action or ""
    frames[#frames + 1] = f
end

--- The episode's result this tick, or nil while it goes on.
local function outcome(ball, rs)
    local t = sim.time() - t0
    if ball.x > GOAL_X and math.abs(ball.y) < GOAL_HALF_WIDTH then
        return "goal"
    end
    if math.abs(ball.x) > FIELD_HALF_X or math.abs(ball.y) > FIELD_HALF_Y then
        local shot = #kicks > 0 and kicks[#kicks].action == "kick_to_goal"
        return (shot and ball.x > 0) and "wide" or "out"
    end
    -- Yellow control: the closest robot is yellow, touching the slow ball, long enough.
    local closest, best = nil, math.huge
    for i, r in ipairs(rs) do
        local d = dist(r, ball)
        if d < best then closest, best = i, d end
    end
    local speed = math.sqrt(ball.vel_x ^ 2 + ball.vel_y ^ 2)
    if closest >= 3 and best <= CONTROL_DIST and speed < CONTROL_SPEED then
        control_since = control_since or sim.time()
        if sim.time() - control_since >= CONTROL_TIME then
            return closest == 4 and "saved" or "stolen"
        end
    else
        control_since = nil
    end
    if t > EPISODE_TIME then
        return "timeout"
    end
    return nil
end

begin(1)

function process()
    -- The kick events of this tick were caused by last tick's commands: attribute them to
    -- the actions chosen then, before the roles change (the passer turns supporter).
    local chosen_before = {}
    for id, c in pairs(ai.chosen) do chosen_before[id] = c end
    ai.process()
    defense.process()

    for _, e in ipairs(sim.events()) do
        if e.kind == "kick" then
            local chosen = e.team == 0 and chosen_before[e.id] or nil
            kicks[#kicks + 1] = {
                t = e.time - t0, id = e.id, team = e.team,
                action = chosen and chosen.action or (e.team == 0 and "?" or "yellow"),
            }
        elseif e.kind == "robot_touched_ball" then
            last_touch = { id = e.id, team = e.team }
        end
    end

    local ball = get_ball_state()
    local rs = robots()
    record_frame(ball, rs)
    local result = outcome(ball, rs)
    if result then
        if finish_episode(result) then
            summary()
            sim.finish(true)
        end
    end
end
