--- lane.lua -- how likely a ball rolling along a segment gets through.
---
--- Pure module: every input is passed in.
---
---     Λ(a, b, E) = Π_e σ( SHARPNESS · (δ_e − CLEARANCE) )
---
--- δ_e is the distance from enemy e to the segment a -> b and σ the logistic
--- function, so each enemy is a soft gate: 0.5 when it just touches the ball's
--- path, ~1 when well clear.

local M = {}

M.ROBOT_RADIUS = 0.09
M.BALL_RADIUS = 0.021
M.SHARPNESS = 12.0  -- logistic steepness for lane clearance

local function dist(a, b)
	return math.sqrt((a.x - b.x) ^ 2 + (a.y - b.y) ^ 2)
end

--- Distance from point c to segment a -> b.
--- @param c { x: number, y: number }
--- @param a { x: number, y: number }
--- @param b { x: number, y: number }
--- @return number
function M.point_to_segment(c, a, b)
	local dx, dy = b.x - a.x, b.y - a.y
	local len2 = dx * dx + dy * dy
	if len2 < 1e-9 then return dist(c, a) end
	local t = ((c.x - a.x) * dx + (c.y - a.y) * dy) / len2
	t = math.max(0.0, math.min(1.0, t))
	return dist(c, { x = a.x + t * dx, y = a.y + t * dy })
end

--- P(ball travels a -> b without an opponent intercepting it), in [0, 1].
--- @param a { x: number, y: number }
--- @param b { x: number, y: number }
--- @param opponents { x: number, y: number }[]
--- @return number
function M.clear(a, b, opponents)
	local p = 1.0
	local clearance = M.ROBOT_RADIUS + M.BALL_RADIUS
	for _, opp in ipairs(opponents) do
		local d = M.point_to_segment(opp, a, b)
		p = p / (1.0 + math.exp(-M.SHARPNESS * (d - clearance)))
	end
	return p
end

return M
