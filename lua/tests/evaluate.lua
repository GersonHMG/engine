-- evaluate: runs an entry script (lua/run_ai.lua by default) on one scenario, N episodes,
-- and prints one line per episode. Run it headless (`engine --headless lua/tests/evaluate.lua`) once per
-- scenario for the working tree and once for the baseline commit, with the same settings,
-- so episode `rep` of a scenario starts from the same place on both sides (a paired design).
--
-- Episode 1 starts exactly as the scenario file says; episode k > 1 moves every robot and
-- the ball by up to JITTER m and turns every robot by up to 2·JITTER rad (own LCG, seeded by
-- SEED, the scenario and k: reproducible). Without jitter every repeat would be the same run.
--
-- Every episode reloads the entry script and the modules it requires (package.loaded is
-- cleared), so no state leaks from one episode to the next.
--
-- Result of an episode (blue attacks x = +4.5):
--   goal      the ball crosses x > 4.5 with |y| < 0.5
--   conceded  the ball crosses x < -4.5 with |y| < 0.5
--   foul      a blue robot touches a defense area (blue's goalkeeper may be in its own)
--   out       the ball leaves the field after a blue kick (or no kick)
--   cleared   the ball leaves the field after a yellow kick
--   saved     the yellow keeper (id 1) controls the ball, or it lies dead in yellow's area
--   stolen    another yellow robot controls the ball
--   own_area  the ball lies dead in blue's area and blue has no goalkeeper
--   timeout   none of the above within EPISODE_TIME
--
-- Settings (environment variables):
--   EVAL_SCENARIO  path of the scenarios/*.toml file (required)
--   EVAL_ENTRY     entry script, default lua/run_ai.lua (relative to the working directory)
--   EVAL_N         episodes, default 10
--   EVAL_TIME      s per episode, default 15
--   EVAL_JITTER    m, default 0.05
--   EVAL_SEED      default 1
--
-- Output, tab separated:  EPISODE <rep> <result> <time s> <blue kicks> <yellow kicks>
-- then                    DONE <episodes>
--
-- Run from the repository root:
--   EVAL_SCENARIO=scenarios/2v2_wing.toml EVAL_N=5 ./target/release/engine --headless --max-time 200 lua/tests/evaluate.lua
package.path = "lua/?.lua;" .. package.path

local function setting(name, default)
    local v = os.getenv(name)
    if v == nil or v == "" then return default end
    return v
end

local SCENARIO = assert(setting("EVAL_SCENARIO", nil), "EVAL_SCENARIO is not set")
local ENTRY = setting("EVAL_ENTRY", "lua/run_ai.lua")
local N = math.max(1, math.floor(tonumber(setting("EVAL_N", "10")) or 10))
local EPISODE_TIME = tonumber(setting("EVAL_TIME", "15")) or 15.0
local JITTER = tonumber(setting("EVAL_JITTER", "0.05")) or 0.05
local SEED = math.floor(tonumber(setting("EVAL_SEED", "1")) or 1)

local GOAL_X = 4.5              -- m
local GOAL_HALF_WIDTH = 0.5     -- m
local FIELD_HALF_X, FIELD_HALF_Y = 4.5, 3.0
local AREA_DEPTH, AREA_HALF_Y = 1.0, 1.0
local ROBOT_RADIUS = 0.09       -- m
local KEEPER_ID = 1             -- yellow's keeper (tests/lib/defense_*)
local CONTROL_DIST = 0.16       -- m, robot center to ball for "controls the ball"
local CONTROL_SPEED = 0.5       -- m/s
local CONTROL_TIME = 0.5        -- s
local DEAD_SPEED = 0.1          -- m/s
local DEAD_TIME = 1.0           -- s

-- ---------------------------------------------------------------------------
-- scenario
-- ---------------------------------------------------------------------------

--- Reads the [ball] / [[robots]] format of scenarios/*.toml.
local function read_scenario(path)
    local f = assert(io.open(path, "r"), "cannot open " .. path)
    local s = { ball = { x = 0, y = 0 }, robots = {} }
    local section, current = nil, nil
    local function flush()
        if current and current.team and current.id and current.x and current.y then
            s.robots[#s.robots + 1] = {
                team = current.team == "yellow" and 1 or 0,
                id = math.floor(current.id),
                x = current.x, y = current.y, theta = current.theta or 0.0,
            }
        end
        current = nil
    end
    for line in f:lines() do
        line = line:gsub("#.*", "")
        if line:match("^%s*%[%[robots%]%]") then
            flush()
            current = {}
            section = "robot"
        elseif line:match("^%s*%[ball%]") then
            flush()
            section = "ball"
        else
            local k, v = line:match("^%s*([%w_]+)%s*=%s*(.-)%s*$")
            if k then
                local value = tonumber(v) or v:gsub('"', "")
                if section == "ball" then
                    s.ball[k] = value
                elseif section == "robot" then
                    current[k] = value
                end
            end
        end
    end
    flush()
    f:close()
    return s
end

local scenario = read_scenario(SCENARIO)
local scenario_name = SCENARIO:match("([^/\\]+)%.toml$") or SCENARIO

local function name_hash(text)
    local h = 5381
    for i = 1, #text do h = (h * 33 + text:byte(i)) % 2147483647 end
    return h
end

local function lcg(seed)
    local s = seed % 2147483647
    if s <= 0 then s = s + 2147483646 end
    return function(a, b)
        s = (s * 48271) % 2147483647
        return a + (b - a) * (s / 2147483647)
    end
end

--- The start of episode `rep`: the scenario, jittered for rep > 1.
local function start_for(rep)
    local rand = lcg(SEED * 1000003 + rep * 7919 + name_hash(scenario_name))
    local j = rep > 1 and JITTER or 0.0
    local s = { ball = { x = scenario.ball.x + rand(-j, j), y = scenario.ball.y + rand(-j, j) }, robots = {} }
    for i, r in ipairs(scenario.robots) do
        s.robots[i] = {
            team = r.team, id = r.id,
            x = r.x + rand(-j, j), y = r.y + rand(-j, j), theta = r.theta + rand(-2 * j, 2 * j),
        }
    end
    return s
end

-- ---------------------------------------------------------------------------
-- entry script, fresh every episode
-- ---------------------------------------------------------------------------

local preloaded = {}
for name in pairs(package.loaded) do preloaded[name] = true end

local entry_process = nil

local function load_entry()
    for name in pairs(package.loaded) do
        if not preloaded[name] then package.loaded[name] = nil end
    end
    -- The entry script defines the global process(); the engine must keep calling ours.
    local harness = process
    process = nil
    dofile(ENTRY)
    entry_process = assert(process, ENTRY .. " defines no process()")
    process = harness
end

-- ---------------------------------------------------------------------------
-- outcome
-- ---------------------------------------------------------------------------

local function dist(a, b)
    return math.sqrt((a.x - b.x) ^ 2 + (a.y - b.y) ^ 2)
end

--- 1 (yellow's area, x > 0), -1 (blue's) or nil: the area a body of radius r at p touches.
local function area_touched(p, r)
    if math.abs(p.y) > AREA_HALF_Y + r then return nil end
    if p.x > FIELD_HALF_X - AREA_DEPTH - r then return 1 end
    if p.x < -(FIELD_HALF_X - AREA_DEPTH - r) then return -1 end
    return nil
end

local blue_count = 0
for _, r in ipairs(scenario.robots) do
    if r.team == 0 then blue_count = blue_count + 1 end
end

--- Whether blue robot `id` is blue's goalkeeper (AI/calculator/role_assigner, when the entry
--- script uses it).
local function is_blue_keeper(id)
    local roles = package.loaded["AI.calculator.role_assigner"]
    return type(roles) == "table" and type(roles.roles) == "table" and roles.roles[id] == "goalkeeper"
end

local rep, t0, control_since, dead_since, last_kick_team, kicks

local function outcome(ball, rs)
    if ball.x > GOAL_X and math.abs(ball.y) < GOAL_HALF_WIDTH then return "goal" end
    if ball.x < -GOAL_X and math.abs(ball.y) < GOAL_HALF_WIDTH then return "conceded" end
    for _, r in ipairs(rs) do
        if r.team == 0 then
            local area = area_touched(r.state, ROBOT_RADIUS)
            if area == 1 or (area == -1 and not is_blue_keeper(r.id)) then return "foul" end
        end
    end
    if math.abs(ball.x) > FIELD_HALF_X or math.abs(ball.y) > FIELD_HALF_Y then
        return last_kick_team == 1 and "cleared" or "out"
    end
    local closest, best = nil, math.huge
    for _, r in ipairs(rs) do
        local d = dist(r.state, ball)
        if d < best then closest, best = r, d end
    end
    local speed = math.sqrt((ball.vel_x or 0) ^ 2 + (ball.vel_y or 0) ^ 2)
    local now = sim.time()
    if closest and closest.team == 1 and best <= CONTROL_DIST and speed < CONTROL_SPEED then
        control_since = control_since or now
        if now - control_since >= CONTROL_TIME then
            return closest.id == KEEPER_ID and "saved" or "stolen"
        end
    else
        control_since = nil
    end
    local area = area_touched(ball, 0.0)
    if area and speed < DEAD_SPEED and (area == 1 or blue_count < 3) then
        dead_since = dead_since or now
        if now - dead_since >= DEAD_TIME then
            return area == 1 and "saved" or "own_area"
        end
    else
        dead_since = nil
    end
    if now - t0 > EPISODE_TIME then return "timeout" end
    return nil
end

-- ---------------------------------------------------------------------------
-- episodes
-- ---------------------------------------------------------------------------

local function begin(index)
    rep = index
    local s = start_for(index)
    for _, r in ipairs(s.robots) do
        grsim.teleport_robot(r.id, r.team, r.x, r.y, r.theta)
    end
    grsim.teleport_ball(s.ball.x, s.ball.y)
    load_entry()
    sim.events()
    t0 = sim.time()
    control_since, dead_since, last_kick_team = nil, nil, nil
    kicks = { [0] = 0, [1] = 0 }
end

begin(1)

function process()
    entry_process()
    for _, e in ipairs(sim.events()) do
        if e.kind == "kick" then
            last_kick_team = e.team
            kicks[e.team] = (kicks[e.team] or 0) + 1
        end
    end
    local rs = {}
    for _, r in ipairs(scenario.robots) do
        rs[#rs + 1] = { team = r.team, id = r.id, state = get_robot_state(r.id, r.team) }
    end
    local result = outcome(get_ball_state(), rs)
    if result then
        print(string.format("EPISODE\t%d\t%s\t%.3f\t%d\t%d", rep, result, sim.time() - t0, kicks[0], kicks[1]))
        if rep >= N then
            print(string.format("DONE\t%d", N))
            sim.finish(true)
        else
            begin(rep + 1)
        end
    end
end
