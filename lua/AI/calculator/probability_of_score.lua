--- probability_of_score.lua -- how likely a shot at a target is to score.
---
--- Pure module: it never reads engine state, every input is passed in. That
--- keeps it testable and lets `AI.calculator.world` own the world snapshot.
---
---     local prob = require("AI.calculator.probability_of_score")
---     local p = prob.calc(robot, goal, world.active_enemies())

local M = {}

-- Tuning knobs.
M.LAMBDA = 0.35        -- how fast the score chance decays with distance (field is ~9 m long)
M.BLOCK_RADIUS = 0.12  -- an enemy this close to the shot line blocks it fully
M.CLEAR_RADIUS = 0.45  -- past this the enemy no longer bothers the shot

--- How open the line from `origin` to `target` is, in [0, 1].
--- 1.0 means nobody stands in the way; 0.0 means a blocker sits on the line.
--- Only blockers *between* origin and target count: one behind the shooter or
--- past the target cannot block anything.
--- @param origin { x: number, y: number }
--- @param target { x: number, y: number }
--- @param blockers { x: number, y: number }[]  -- usually the active enemies
--- @return number
function M.clearance(origin, target, blockers)
	local dx = target.x - origin.x
	local dy = target.y - origin.y
	local length = math.sqrt(dx * dx + dy * dy)

	if length < 0.001 then
		return 0.0
	end

	-- Unit vector pointing at the target.
	local dir_x = dx / length
	local dir_y = dy / length

	local openness = 1.0

	for _, blocker in ipairs(blockers) do
		local bx = blocker.x - origin.x
		local by = blocker.y - origin.y

		-- How far along the line the blocker sits (dot product).
		local along = dir_x * bx + dir_y * by

		if along > 0.0 and along < length then
			-- Perpendicular distance to the line (2D cross product).
			local offset = math.abs(dir_x * by - dir_y * bx)

			local factor
			if offset <= M.BLOCK_RADIUS then
				factor = 0.0
			elseif offset >= M.CLEAR_RADIUS then
				factor = 1.0
			else
				factor = (offset - M.BLOCK_RADIUS) / (M.CLEAR_RADIUS - M.BLOCK_RADIUS)
			end

			-- The worst blocker decides how open the line is.
			if factor < openness then
				openness = factor
			end
		end
	end

	return openness
end

--- Distance term only: closer to the target is a better shot.
--- @param origin { x: number, y: number }
--- @param target { x: number, y: number }
--- @return number  -- in (0, 1]
function M.range_factor(origin, target)
	local dx = target.x - origin.x
	local dy = target.y - origin.y
	return math.exp(-M.LAMBDA * math.sqrt(dx * dx + dy * dy))
end

--- P(score | shooting at `target` from `origin`) = range * clearance.
--- @param origin { x: number, y: number }   -- the shooter
--- @param target { x: number, y: number }   -- aim point, e.g. the goal center
--- @param blockers { x: number, y: number }[]
--- @return number  -- in [0, 1]
function M.calc(origin, target, blockers)
	return M.range_factor(origin, target) * M.clearance(origin, target, blockers)
end

return M
