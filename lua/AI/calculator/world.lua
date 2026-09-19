--- world.lua -- central access point for all world data.
---
--- Every other module should read ball / robot state through this module
--- instead of calling the engine globals (`get_ball_state`,
--- `get_blue_team_state`, `get_yellow_team_state`, `get_robot_state`)
--- directly.
---
--- Usage, once per tick:
---     local world = require("AI.calculator.world")
---     function process()
---         world.update(0)            -- your team id
---         local ball = world.ball()
---         local mine = world.active_allies()
---     end
---
--- `update` takes one snapshot of the engine state, so every read inside the
--- same tick sees consistent data and the engine is queried only once.

local probability_of_score = require("AI.calculator.probability_of_score")

local M = {}

-- ---------------------------------------------------------------------------
-- snapshot state
-- ---------------------------------------------------------------------------

M.tick = 0            -- number of completed updates
M.team = nil          -- ally team id (0 = blue, 1 = yellow)
M.enemy_team = nil    -- opponent team id

M.ball_state = nil    -- BallState
M.ally_robots = {}    -- RobotState[]
M.enemy_robots = {}   -- RobotState[]
M.ally_by_id = {}     -- table<integer, RobotState>
M.enemy_by_id = {}    -- table<integer, RobotState>
M.ally_active = {}    -- RobotState[] (active only)
M.enemy_active = {}   -- RobotState[] (active only)

-- ---------------------------------------------------------------------------
-- internals
-- ---------------------------------------------------------------------------

---@param team integer
---@return RobotState[]
local function team_state(team)
	return (team == 0) and get_blue_team_state() or get_yellow_team_state()
end

---Builds the id -> robot map and the active-only list for one team.
---@param robots RobotState[]
---@return table<integer, RobotState> by_id
---@return RobotState[] active
local function index(robots)
	local by_id, active = {}, {}
	for _, r in ipairs(robots) do
		by_id[r.id] = r
		if r.active then
			active[#active + 1] = r
		end
	end
	return by_id, active
end

local function assert_ready()
	if not M.ball_state then
		error("AI.calculator.world: call world.update(team) before reading state", 3)
	end
end

-- ---------------------------------------------------------------------------
-- snapshot
-- ---------------------------------------------------------------------------

---Takes a fresh snapshot of the world. Call once at the top of `process()`.
---@param team integer  -- your team id (0 = blue, 1 = yellow)
function M.update(team)
	M.team = team
	M.enemy_team = (team == 0) and 1 or 0

	M.ball_state = get_ball_state()
	M.ally_robots = team_state(M.team)
	M.enemy_robots = team_state(M.enemy_team)

	M.ally_by_id, M.ally_active = index(M.ally_robots)
	M.enemy_by_id, M.enemy_active = index(M.enemy_robots)

	M.tick = M.tick + 1
end

-- ---------------------------------------------------------------------------
-- ball
-- ---------------------------------------------------------------------------

---@return BallState
function M.ball()
	assert_ready()
	return M.ball_state
end

---@return Vec2
function M.ball_pos()
	assert_ready()
	return { x = M.ball_state.x, y = M.ball_state.y }
end

---@return Vec2
function M.ball_vel()
	assert_ready()
	return { x = M.ball_state.vel_x, y = M.ball_state.vel_y }
end

---@return number  -- m/s
function M.ball_speed()
	assert_ready()
	local vx, vy = M.ball_state.vel_x, M.ball_state.vel_y
	return math.sqrt(vx * vx + vy * vy)
end

-- ---------------------------------------------------------------------------
-- robots
-- ---------------------------------------------------------------------------

---@return RobotState[]  -- all of your robots, active or not
function M.allies()
	assert_ready()
	return M.ally_robots
end

---@return RobotState[]  -- all opponent robots, active or not
function M.enemies()
	assert_ready()
	return M.enemy_robots
end

---@return RobotState[]  -- your robots currently on the field
function M.active_allies()
	assert_ready()
	return M.ally_active
end

---@return RobotState[]  -- opponent robots currently on the field
function M.active_enemies()
	assert_ready()
	return M.enemy_active
end

---Looks up one robot from the snapshot.
---@param id integer
---@param team? integer  -- defaults to your team
---@return RobotState|nil
function M.robot(id, team)
	assert_ready()
	team = team or M.team
	if team == M.team then
		return M.ally_by_id[id]
	end
	return M.enemy_by_id[id]
end

---@param id integer
---@return RobotState|nil
function M.ally(id)
	assert_ready()
	return M.ally_by_id[id]
end

---@param id integer
---@return RobotState|nil
function M.enemy(id)
	assert_ready()
	return M.enemy_by_id[id]
end

-- ---------------------------------------------------------------------------
-- geometry helpers (accept anything with .x / .y: RobotState, BallState, Vec2)
-- ---------------------------------------------------------------------------

---@param a { x: number, y: number }
---@param b { x: number, y: number }
---@return number
function M.distance(a, b)
	local dx = a.x - b.x
	local dy = a.y - b.y
	return math.sqrt(dx * dx + dy * dy)
end

---@param entity { x: number, y: number }
---@return number
function M.distance_to_ball(entity)
	assert_ready()
	return M.distance(entity, M.ball_state)
end

---Angle of the vector a -> b, in radians.
---@param a { x: number, y: number }
---@param b { x: number, y: number }
---@return number
function M.angle_between(a, b)
	return math.atan(b.y - a.y, b.x - a.x)
end

-- ---------------------------------------------------------------------------
-- derived queries
-- ---------------------------------------------------------------------------

---Active robots of one side ranked by distance to the ball, closest first.
---@param robots RobotState[]
---@return { robot: RobotState, distance: number }[]
local function rank_by_ball(robots)
	local ordered = {}
	for _, r in ipairs(robots) do
		ordered[#ordered + 1] = {
			robot = r,
			distance = M.distance(r, M.ball_state),
		}
	end
	table.sort(ordered, function(a, b)
		return a.distance < b.distance
	end)
	return ordered
end

---@return { robot: RobotState, distance: number }[]  -- your robots, closest to the ball first
function M.allies_by_ball_distance()
	assert_ready()
	return rank_by_ball(M.ally_active)
end

---@return { robot: RobotState, distance: number }[]  -- opponents, closest to the ball first
function M.enemies_by_ball_distance()
	assert_ready()
	return rank_by_ball(M.enemy_active)
end

---@return RobotState|nil  -- your robot closest to the ball
function M.closest_ally_to_ball()
	local first = M.allies_by_ball_distance()[1]
	return first and first.robot or nil
end

---@return RobotState|nil  -- opponent closest to the ball
function M.closest_enemy_to_ball()
	local first = M.enemies_by_ball_distance()[1]
	return first and first.robot or nil
end

---Closest robot to the ball across both teams. Ties go to your team.
---@return RobotState|nil
function M.nearest_to_ball()
	local ally = M.allies_by_ball_distance()[1]
	local enemy = M.enemies_by_ball_distance()[1]

	if not ally and not enemy then
		return nil
	end
	if not ally then
		return enemy.robot
	end
	if not enemy then
		return ally.robot
	end

	if ally.distance <= enemy.distance then
		return ally.robot
	end
	return enemy.robot
end

---How open the line from `origin` to `target` is against the active enemies,
---in [0, 1]. 1.0 = nobody in the way, 0.0 = an enemy sits on the line.
---@param origin { x: number, y: number }
---@param target { x: number, y: number }
---@return number
function M.shot_clearance(origin, target)
	assert_ready()
	return probability_of_score.clearance(origin, target, M.enemy_active)
end

---P(score | shooting at `target` from `origin`), given the current enemies.
---@param origin { x: number, y: number }   -- the shooter
---@param target { x: number, y: number }   -- aim point, e.g. the goal center
---@return number  -- in [0, 1]
function M.probability_of_score(origin, target)
	assert_ready()
	return probability_of_score.calc(origin, target, M.enemy_active)
end

---@return boolean  -- true when one of your robots is the closest to the ball
function M.team_has_ball_advantage()
	local nearest = M.nearest_to_ball()
	return nearest ~= nil and nearest.team == M.team
end

return M
