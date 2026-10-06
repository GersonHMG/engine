--- role_assigner.lua -- which role each of our robots plays this tick.
---
---   goalkeeper  with KEEPER_MIN_ROBOTS or more robots: the one nearest our goal, kept
---               while it is on the field (it never chases the ball)
---   attacker    the ball owner (ball_owner, among the field robots)
---   supporter   up to MAX_SUPPORTERS: the pass receiver while a pass is in flight, else
---               the most advanced robot, kept while it is on the field (one more while
---               a pass is in flight and nobody is attacker: the passer)
---   defender    everyone else; each marks an enemy (mark_targets) or guards the ball
---
--- Call `assign(robots)` once per tick, after `world.update`; it runs ball_owner.update.

local world = require("AI.calculator.world")
local ball_owner = require("AI.calculator.ball_owner")

local M = {}

M.OWN_GOAL = { x = -4.5, y = 0.0 }
M.KEEPER_MIN_ROBOTS = 3  -- fewer robots than this: no goalkeeper, everyone plays the field
M.MAX_SUPPORTERS = 1     -- more supporters would all look for the same open spot

local keeper_id = nil      -- remembered across ticks
local supporter_ids = {}   -- remembered across ticks: id -> true

M.roles = {}         -- table<integer, string>, robot id -> role name, this tick
M.mark_targets = {}  -- table<integer, integer>, defender id -> enemy id to mark
M.guard_index = {}   -- table<integer, integer>, defender id -> 1, 2, ... for defenders with nobody to mark

local function by_id(robots, id)
	for _, r in ipairs(robots) do
		if r.id == id then return r end
	end
	return nil
end

--- Enemies worth marking, most dangerous (nearest our goal) first; the enemy nearest the
--- ball is left to the attacker.
local function threats()
	local ball = world.ball()
	local on_ball, best = nil, math.huge
	for _, e in ipairs(world.active_enemies()) do
		local d = world.distance(e, ball)
		if d < best then on_ball, best = e, d end
	end
	local enemies = {}
	for _, e in ipairs(world.active_enemies()) do
		if e ~= on_ball then enemies[#enemies + 1] = e end
	end
	table.sort(enemies, function(a, b)
		return world.distance(a, M.OWN_GOAL) < world.distance(b, M.OWN_GOAL)
	end)
	return enemies
end

--- The goalkeeper: kept while on the field, else the robot nearest our goal.
local function pick_keeper(robots)
	if #robots < M.KEEPER_MIN_ROBOTS then
		return nil
	end
	if keeper_id and by_id(robots, keeper_id) then
		return keeper_id
	end
	local best, best_d = nil, math.huge
	for _, r in ipairs(robots) do
		local d = world.distance(r, M.OWN_GOAL)
		if d < best_d then best, best_d = r, d end
	end
	return best and best.id
end

--- Assigns the roles for this tick.
--- @param robots RobotState[]  -- our active robots
function M.assign(robots)
	M.roles, M.mark_targets, M.guard_index = {}, {}, {}

	-- Goalkeeper
	keeper_id = pick_keeper(robots)
	local field = {}
	for _, r in ipairs(robots) do
		if r.id == keeper_id then
			M.roles[r.id] = "goalkeeper"
		else
			field[#field + 1] = r
		end
	end

	-- Attacker
	ball_owner.update(field)
	if ball_owner.owner then
		M.roles[ball_owner.owner.id] = "attacker"
	end

	-- Supporters: the pass receiver first, then the remembered ones, then the most advanced.
	-- While a pass is in flight nobody is attacker: the passer keeps a supporter slot too.
	local slots = M.MAX_SUPPORTERS + (ball_owner.owner and 0 or 1)
	local kept = {}
	local function add_supporter(r)
		if r and #kept < slots and not M.roles[r.id] then
			M.roles[r.id] = "supporter"
			kept[#kept + 1] = r.id
		end
	end
	add_supporter(ball_owner.receiver)
	for _, r in ipairs(field) do
		if supporter_ids[r.id] then add_supporter(r) end
	end
	local rest = {}
	for _, r in ipairs(field) do
		if not M.roles[r.id] then rest[#rest + 1] = r end
	end
	table.sort(rest, function(a, b) return a.x > b.x end)
	for _, r in ipairs(rest) do add_supporter(r) end
	supporter_ids = {}
	for _, id in ipairs(kept) do supporter_ids[id] = true end

	-- Defenders: each threat gets the nearest free defender; the rest guard the ball.
	local free = {}
	for _, r in ipairs(field) do
		if not M.roles[r.id] then
			M.roles[r.id] = "defender"
			free[#free + 1] = r
		end
	end
	for _, enemy in ipairs(threats()) do
		if #free == 0 then break end
		local best_i, best_d = nil, math.huge
		for i, d in ipairs(free) do
			local dist = world.distance(d, enemy)
			if dist < best_d then best_i, best_d = i, dist end
		end
		M.mark_targets[free[best_i].id] = enemy.id
		table.remove(free, best_i)
	end
	for i, d in ipairs(free) do
		M.guard_index[d.id] = i
	end
end

return M
