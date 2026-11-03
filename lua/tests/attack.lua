-- attack: how often the blue AI (lua/AI) scores against a fixed yellow defense.
--
-- Objective: win rate = episodes with a goal / episodes.
--   win   the ball crosses x > 4.5 with |y| < GOAL_HALF_WIDTH within EPISODE_TIME
--   loss  timeout; ball out of the field (`wide` after a shot); a yellow robot controls
--         the ball (closest robot, within CONTROL_DIST, ball slower than CONTROL_SPEED for
--         CONTROL_TIME: `saved` by the keeper, else `stolen`); or `foul`: a blue robot
--         that is not blue's goalkeeper touches a defense area with its body, or blue's
--         goalkeeper touches the enemy's (only goalkeepers may enter their own area).
--
-- Modes (yellow plays tests/lib/defense_<mode>.lua, the same code as run_ai.lua for 2v2):
--   2v2  blue 0, 1 vs yellow 0 (marks the blue robot away from the ball), 1 (keeper)
--   3v3  blue 0, 1, 2 (role_assigner makes the one nearest our goal keeper) vs yellow 0
--        (marker), 1 (keeper), 2 (guards the ball)
-- Random starts (own LCG, reproducible): ball x in [0, 3.5], |y| <= 2.5; blue 0 near the
-- ball, blue 1 anywhere in the attacking half, blue 2 (3v3) in our own defense area; yellow
-- on the spots their defense sends them to; everything at rest.
--
-- Settings (environment variables, or globals set by a wrapper script):
--   MODE      2v2 (default) | 3v3
--   SEEDS     tuning (default) | holdout | named (the scenarios/<mode>_*.toml files and,
--             for 2v2, scenarios/2vs1.toml and marked_pass.toml)
--   N         episodes (default 100)
--   RECORD    path of a JSON file with the trajectories of the first RECORD_N episodes
--   RECORD_N  default 12
--
-- Run from the repository root:
--   MODE=3v3 SEEDS=holdout ./target/release/engine --headless --max-time 4000 lua/tests/attack.lua
package.path = "lua/?.lua;" .. package.path

local ai = require("AI.main")
local role_assigner = require("AI.calculator.role_assigner")
local profile = require("utils.robot_profile")
local mark = require("skills.mark")
local goalkeeper = require("skills.goalkeeper")

local EPISODE_TIME = 15.0       -- s
local GOAL_X = 4.5              -- m, the goal line blue attacks
local GOAL_HALF_WIDTH = 0.5     -- m
local FIELD_HALF_X, FIELD_HALF_Y = 4.5, 3.0  -- m
local AREA_DEPTH, AREA_HALF_Y = 1.0, 1.0     -- m, defense areas (SSL division B)
local CONTROL_DIST = 0.16       -- m, robot center to ball for "controls the ball"
local CONTROL_SPEED = 0.5       -- m/s
local CONTROL_TIME = 0.5        -- s
local DEAD_SPEED = 0.1          -- m/s, a ball this slow inside a defense area is lost to us
local DEAD_TIME = 1.0           -- s
local MIN_GAP = 0.3             -- m between robots at the start
local RECORD_EVERY = 2          -- ticks between recorded frames

local function setting(name, default)
    return os.getenv(name) or _G["ATTACK_" .. name] or default
end

local MODE = setting("MODE", "2v2")
local SET = setting("SEEDS", "tuning")
local N = tonumber(setting("N", "")) or 100
local RECORD = setting("RECORD", nil)
local RECORD_N = tonumber(setting("RECORD_N", "")) or 12
local BASE = ({ tuning = 1, holdout = 2, named = 3 })[SET] or 1

local defense = require("tests.lib.defense_" .. MODE)
-- The opponent model (robot profile), as scouted: only the 3v3 defense guards the ball.
profile.opponent.guards_ball = (MODE == "3v3")
local BLUE_IDS = MODE == "3v3" and { 0, 1, 2 } or { 0, 1 }
local YELLOW_IDS = MODE == "3v3" and { 0, 1, 2 } or { 0, 1 }

--- Every robot of the episode, in a fixed order (frames list them in this order).
local ROBOTS = {}
for _, id in ipairs(BLUE_IDS) do ROBOTS[#ROBOTS + 1] = { team = 0, id = id } end
for _, id in ipairs(YELLOW_IDS) do ROBOTS[#ROBOTS + 1] = { team = 1, id = id } end

-- ---------------------------------------------------------------------------
-- geometry
-- ---------------------------------------------------------------------------

local function dist(a, b)
    return math.sqrt((a.x - b.x) ^ 2 + (a.y - b.y) ^ 2)
end

local function facing(from, to)
    return math.atan(to.y - from.y, to.x - from.x)
end

--- Which defense area a body of radius `r` at `p` touches: 1 (yellow's, x > 0), -1
--- (blue's), or nil.
local function area_touched(p, r)
    if math.abs(p.y) > AREA_HALF_Y + r then return nil end
    if p.x > FIELD_HALF_X - AREA_DEPTH - r then return 1 end
    if p.x < -(FIELD_HALF_X - AREA_DEPTH - r) then return -1 end
    return nil
end

-- ---------------------------------------------------------------------------
-- starts: { ball = {x, y}, robots = { [team .. ":" .. id] = {x, y, theta?} } }
-- ---------------------------------------------------------------------------

local function key(team, id) return team .. ":" .. id end

local function lcg(seed)
    local s = seed % 2147483647
    if s <= 0 then s = s + 2147483646 end
    return function(a, b)
        s = (s * 48271) % 2147483647
        return a + (b - a) * (s / 2147483647)
    end
end

--- Places the defense the way it plays (marker, keeper, guard on their spots).
local function place_defense(s)
    local ball = s.ball
    local field = {}
    for _, id in ipairs({ 0, 1 }) do field[#field + 1] = s.robots[key(0, id)] end
    local free = dist(field[1], ball) >= dist(field[2], ball) and field[1] or field[2]
    s.robots[key(1, 0)] = mark.spot(free, ball)
    s.robots[key(1, 1)] = goalkeeper.spot(ball, defense.GOAL)
    if MODE == "3v3" then
        s.robots[key(1, 2)] = defense.guard_spot(ball)
    end
end

local function valid(s)
    local list = {}
    for _, r in ipairs(ROBOTS) do
        local p = s.robots[key(r.team, r.id)]
        list[#list + 1] = p
        if math.abs(p.x) > FIELD_HALF_X - 0.1 or math.abs(p.y) > FIELD_HALF_Y - 0.1 then return false end
        if dist(p, s.ball) < 0.2 then return false end
        -- Only blue 2 (3v3 keeper) and yellow 1 (keeper) may start in an area.
        local keeper = (r.team == 0 and r.id == 2) or (r.team == 1 and r.id == 1)
        if not keeper and area_touched(p, profile.robot.radius) then return false end
    end
    if area_touched(s.ball, 0.0) then return false end
    for i = 1, #list do
        for j = i + 1, #list do
            if dist(list[i], list[j]) < MIN_GAP then return false end
        end
    end
    return true
end

local function random_start(index)
    local rand = lcg(BASE * 100003 + index * 7919 + (MODE == "3v3" and 55 or 0))
    for _ = 1, 500 do
        local s = { ball = { x = rand(0.0, 3.5), y = rand(-2.5, 2.5) }, robots = {} }
        local a, d = rand(-math.pi, math.pi), rand(0.5, 1.5)
        s.robots[key(0, 0)] = { x = s.ball.x + d * math.cos(a), y = s.ball.y + d * math.sin(a) }
        s.robots[key(0, 1)] = { x = rand(0.0, 3.5), y = rand(-2.5, 2.5) }
        if MODE == "3v3" then
            s.robots[key(0, 2)] = { x = -4.25, y = rand(-0.5, 0.5) }
        end
        place_defense(s)
        if valid(s) then return s end
    end
    error("no valid start for episode " .. index)
end

--- Reads the [ball] / [[robots]] format of scenarios/*.toml.
local function read_scenario(path)
    local f = assert(io.open(path, "r"))
    local s = { name = path:match("([^/]+)%.toml$"), robots = {} }
    local section, current = nil, nil
    for line in f:lines() do
        line = line:gsub("#.*", "")
        if line:match("^%s*%[%[robots%]%]") then
            current = {}
            section = "robot"
        elseif line:match("^%s*%[ball%]") then
            s.ball = {}
            section = "ball"
        else
            local k, v = line:match("^%s*([%w_]+)%s*=%s*(.-)%s*$")
            if k then
                local value = tonumber(v) or v:gsub('"', "")
                if section == "ball" then
                    s.ball[k] = value
                elseif section == "robot" then
                    current[k] = value
                    if current.team and current.id and current.x and current.y and current.theta then
                        local team = current.team == "yellow" and 1 or 0
                        s.robots[key(team, current.id)] = { x = current.x, y = current.y, theta = current.theta }
                        current = {}
                    end
                end
            end
        end
    end
    f:close()
    return s
end

local NAMED_FILES = {
    ["2v2"] = { "2vs1", "marked_pass", "2v2_far_start", "2v2_wing", "2v2_near_area" },
    ["3v3"] = { "3v3_counter", "3v3_guarded" },
}
local named = {}
if SET == "named" then
    for _, name in ipairs(NAMED_FILES[MODE]) do
        named[#named + 1] = read_scenario("scenarios/" .. name .. ".toml")
    end
    N = #named
end

local function start_for(index)
    if SET == "named" then return named[index] end
    return random_start(index)
end

-- ---------------------------------------------------------------------------
-- episode state
-- ---------------------------------------------------------------------------

local episode, start, t0
local control_since, dead_since, kicks
local results, counts = {}, {}
local recorded, frames, tick = {}, nil, 0
local yellow_touches = 0  -- ticks a yellow non-keeper touched a defense area (count_yellow)

local function begin(index)
    episode = index
    start = start_for(index)
    for _, r in ipairs(ROBOTS) do
        local p = start.robots[key(r.team, r.id)]
        grsim.teleport_robot(r.id, r.team, p.x, p.y, p.theta or facing(p, start.ball))
    end
    grsim.teleport_ball(start.ball.x, start.ball.y)
    ai.reset()
    sim.events()
    t0 = sim.time()
    control_since, dead_since, kicks = nil, nil, {}
    frames = (RECORD and index <= RECORD_N) and {} or nil
    tick = 0
end

local function finish_episode(result, detail)
    local t = sim.time() - t0
    counts[result] = (counts[result] or 0) + 1
    local kick_names = {}
    for _, k in ipairs(kicks) do kick_names[#kick_names + 1] = k.action end
    results[#results + 1] = { index = episode, result = result, time = t }
    print(string.format("%-3d %-9s t=%5.2f s  kicks=[%s]%s%s", episode, result, t,
        table.concat(kick_names, ","), detail and ("  " .. detail) or "",
        start.name and ("  " .. start.name) or ""))
    if frames then
        recorded[#recorded + 1] = {
            index = episode, name = start.name, result = result, time = t, kicks = kicks, frames = frames,
        }
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
        return (string.format("%.3f", value):gsub("%.?0+$", ""))
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
    print(string.format("SUMMARY mode=%s set=%s win=%d/%d (%.1f%%, 95%% CI %.1f-%.1f%%) %s  yellow_area_ticks=%d",
        MODE, SET, wins, n, 100 * p, 100 * (centre - half), 100 * (centre + half), table.concat(hist, " "),
        yellow_touches))
    if RECORD then
        local f = assert(io.open(RECORD, "w"))
        f:write(json({ mode = MODE, set = SET, win = wins, n = n, robots = ROBOTS, episodes = recorded }))
        f:close()
    end
end

-- ---------------------------------------------------------------------------
-- tick
-- ---------------------------------------------------------------------------

local function robot_states()
    local rs = {}
    for i, r in ipairs(ROBOTS) do rs[i] = get_robot_state(r.id, r.team) end
    return rs
end

local function record_frame(ball, rs)
    tick = tick + 1
    if not frames or tick % RECORD_EVERY ~= 1 then return end
    local f = { sim.time() - t0, ball.x, ball.y }
    for _, r in ipairs(rs) do
        f[#f + 1] = r.x; f[#f + 1] = r.y; f[#f + 1] = r.orientation
    end
    for _, id in ipairs(BLUE_IDS) do
        local c = ai.chosen[id]
        f[#f + 1] = c and c.action or ""
    end
    frames[#frames + 1] = f
end

--- Yellow robots other than the keeper touching a defense area this tick (the rule holds
--- for both teams; yellow is not ours to fix, so it is only counted).
local function count_yellow(rs)
    for i, r in ipairs(ROBOTS) do
        if r.team == 1 and r.id ~= defense.KEEPER_ID and area_touched(rs[i], profile.robot.radius) then
            yellow_touches = yellow_touches + 1
        end
    end
end

--- A blue robot breaking the defense-area rule this tick, as "id:area", or nil.
local function foul(rs)
    for i, r in ipairs(ROBOTS) do
        if r.team == 0 then
            local area = area_touched(rs[i], profile.robot.radius)
            local keeper = role_assigner.roles[r.id] == "goalkeeper"
            if area == 1 or (area == -1 and not keeper) then
                return string.format("blue %d in %s area at (%.2f, %.2f)", r.id,
                    area == 1 and "yellow" or "blue", rs[i].x, rs[i].y)
            end
        end
    end
    return nil
end

--- The episode's result this tick (and a detail), or nil while it goes on.
local function outcome(ball, rs)
    local t = sim.time() - t0
    if ball.x > GOAL_X and math.abs(ball.y) < GOAL_HALF_WIDTH then
        return "goal"
    end
    local f = foul(rs)
    if f then
        return "foul", f
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
    local c = ROBOTS[closest]
    if c.team == 1 and best <= CONTROL_DIST and speed < CONTROL_SPEED then
        control_since = control_since or sim.time()
        if sim.time() - control_since >= CONTROL_TIME then
            return (c.id == defense.KEEPER_ID) and "saved" or "stolen"
        end
    else
        control_since = nil
    end
    -- A slow ball in a defense area: the yellow keeper's (`saved`), or in our own area,
    -- where no field robot of ours may go (`own_area`; 3v3: our keeper may, so it goes on).
    local area = area_touched(ball, 0.0)
    local our_keeper = MODE == "3v3"
    if area and speed < DEAD_SPEED and (area == 1 or not our_keeper) then
        dead_since = dead_since or sim.time()
        if sim.time() - dead_since >= DEAD_TIME then
            return area == 1 and "saved" or "own_area"
        end
    else
        dead_since = nil
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
                score = chosen and chosen.score or nil,
            }
        end
    end

    local ball = get_ball_state()
    local rs = robot_states()
    record_frame(ball, rs)
    count_yellow(rs)
    local result, detail = outcome(ball, rs)
    if result and not detail then
        detail = string.format("ball (%.2f, %.2f)", ball.x, ball.y)
    end
    if result then
        if finish_episode(result, detail) then
            summary()
            sim.finish(true)
        end
    end
end
