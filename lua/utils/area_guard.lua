-- area_guard: keeps our robots out of the defense (goalkeeper) areas.
--
-- Rule: only a goalkeeper may enter a defense area, and only its own. A robot breaks it
-- as soon as any part of its body is inside, but the engine's path planner (move_to) only
-- keeps the robot's center out, and move_direct ignores the areas. So, once installed,
-- every move_to / move_direct of our team goes through `safe_target` (and, near an area,
-- is driven straight with move_direct: the path planner may steer into it):
--   * a target inside a forbidden area, grown by MARGIN, is pushed to its nearest edge;
--   * a straight path that would cut through a forbidden area goes via its nearest corner;
--   * a robot already inside is sent straight out.
-- Other teams' commands are passed through unchanged.
--
--     local guard = require("utils.area_guard")
--     guard.install(team, function(id) return is_goalkeeper(id) end)

local profile = require("utils.robot_profile")

local M = {}

M.FIELD_HALF_X = 4.5    -- m
M.AREA_DEPTH = 1.0      -- m, SSL division B defense area
M.AREA_HALF_Y = 1.0     -- m
M.MARGIN = profile.robot.radius + 0.08  -- m, robot body plus room to brake
M.BRAKE_DECEL = 4.0     -- m/s^2, braking assumed when predicting where a moving robot stops
M.BRAKE_ZONE = 0.5      -- m beyond MARGIN where a robot's stopping point is checked
M.CROWD_DIST = 0.25     -- m, another robot this close (center to center) can push ours
M.CROWD_MARGIN = 0.10   -- m, extra room kept from the areas while another robot is that close
M.OUTSIDE = 1.0         -- m, how far behind the goal line an area extends (covers the goal)

--- The areas, as rectangles grown by `margin`. side = 1: the +x area, -1: the -x one.
--- @param side integer
--- @param margin number
--- @return { x0: number, x1: number, y0: number, y1: number }
function M.area(side, margin)
	local inner = M.FIELD_HALF_X - M.AREA_DEPTH - margin
	local outer = M.FIELD_HALF_X + M.OUTSIDE
	return {
		x0 = side > 0 and inner or -outer,
		x1 = side > 0 and outer or -inner,
		y0 = -M.AREA_HALF_Y - margin,
		y1 = M.AREA_HALF_Y + margin,
	}
end

local function inside(p, a)
	return p.x > a.x0 and p.x < a.x1 and p.y > a.y0 and p.y < a.y1
end

--- True when a body of radius `radius` at `p` touches a defense area.
--- @param p { x: number, y: number }
--- @param radius? number  -- default 0 (a point)
--- @return integer|nil side  -- 1 or -1, nil when clear of both
function M.touched(p, radius)
	for _, side in ipairs({ 1, -1 }) do
		if inside(p, M.area(side, radius or 0)) then return side end
	end
	return nil
end

--- The nearest point on the boundary of `a` from `p` (inside it), never behind the goal.
local function push_out(p, a, side)
	local front = side > 0 and a.x0 or a.x1
	local candidates = {
		{ x = front, y = p.y },
		{ x = p.x, y = a.y0 },
		{ x = p.x, y = a.y1 },
	}
	local best, best_d = candidates[1], math.huge
	for _, c in ipairs(candidates) do
		if math.abs(c.x) <= M.FIELD_HALF_X then
			local d = (c.x - p.x) ^ 2 + (c.y - p.y) ^ 2
			if d < best_d then best, best_d = c, d end
		end
	end
	return best
end

--- Whether segment a -> b crosses rectangle r (Liang-Barsky clipping).
local function crosses(a, b, r)
	local t0, t1 = 0.0, 1.0
	local dx, dy = b.x - a.x, b.y - a.y
	local checks = {
		{ -dx, a.x - r.x0 }, { dx, r.x1 - a.x }, { -dy, a.y - r.y0 }, { dy, r.y1 - a.y },
	}
	for _, c in ipairs(checks) do
		local p, q = c[1], c[2]
		if math.abs(p) < 1e-12 then
			if q < 0 then return false end
		else
			local t = q / p
			if p < 0 then
				if t > t1 then return false end
				if t > t0 then t0 = t end
			else
				if t < t0 then return false end
				if t < t1 then t1 = t end
			end
		end
	end
	return t0 < t1
end

--- Whether segment a -> b crosses defense area `side` grown by `margin`.
--- @return boolean
function M.segment_crosses(a, b, side, margin)
	return crosses(a, b, M.area(side, margin))
end

--- The target to drive to instead of `target`, so the robot at `from` stays out of the
--- areas in `forbidden` (a list of sides).
--- @param from { x: number, y: number }
--- @param target { x: number, y: number }
--- @param forbidden integer[]
--- @return { x: number, y: number }
function M.safe_target(from, target, forbidden, margin)
	margin = margin or M.MARGIN
	-- Where the robot stops if it brakes now: moving towards an area, that point is what
	-- has to stay out.
	local stop = from
	if from.vel_x then
		local v2 = from.vel_x ^ 2 + from.vel_y ^ 2
		local k = 1.0 / (2 * M.BRAKE_DECEL)
		stop = { x = from.x + from.vel_x * math.sqrt(v2) * k, y = from.y + from.vel_y * math.sqrt(v2) * k }
	end
	for _, side in ipairs(forbidden) do
		local a = M.area(side, margin)
		if inside(from, a) then
			return push_out(from, a, side)
		end
		local near = inside(from, M.area(side, margin + M.BRAKE_ZONE))
		if near and inside(stop, a) then
			-- Heading in too fast: stop on the edge nearest to where it would stop.
			local edge = push_out(stop, a, side)
			if not crosses(from, edge, M.area(side, profile.robot.radius)) then
				return edge
			end
		end
		if inside(target, a) then
			target = push_out(target, a, side)
		end
		if crosses(from, target, a) then
			-- Go round the near corner of the front edge on the target's side.
			local front = side > 0 and a.x0 or a.x1
			local y = (target.y + from.y >= 0) and a.y1 or a.y0
			return { x = front, y = y }
		end
	end
	return target
end

local installed = false

--- Every active robot on the field except robot `id` of team `team`.
local function crowd(id, team)
	local out = {}
	for _, t in ipairs({ 0, 1 }) do
		local robots = (t == 0) and get_blue_team_state() or get_yellow_team_state()
		for _, r in ipairs(robots) do
			if r.active and not (t == team and r.id == id) then out[#out + 1] = r end
		end
	end
	return out
end

--- Wraps the engine's move_to and move_direct for `team`.
--- @param team integer
--- @param is_goalkeeper fun(id: integer): boolean
function M.install(team, is_goalkeeper)
	if installed then return end
	installed = true
	local own_side = -1  -- we attack +x: our own area is at -x
	local raw_direct = _G.move_direct
	for _, name in ipairs({ "move_to", "move_direct" }) do
		local raw = _G[name]
		_G[name] = function(id, t, point, ...)
			if t == team and point then
				local robot = get_robot_state(id, t)
				if robot then
					local forbidden = is_goalkeeper(id) and { -own_side } or { 1, -1 }
					-- Another robot touching ours can push it in: keep more room then.
					local margin = M.MARGIN
					for _, other in ipairs(crowd(id, t)) do
						if (other.x - robot.x) ^ 2 + (other.y - robot.y) ^ 2 < M.CROWD_DIST ^ 2 then
							margin = M.MARGIN + M.CROWD_MARGIN
							break
						end
					end
					point = M.safe_target(robot, point, forbidden, margin)
					-- Near a forbidden area, drive straight to the safe target: the path
					-- planner steers around the ball and robots, and a robot that is
					-- already touching the ball gets steered into the area.
					for _, side in ipairs(forbidden) do
						if inside(robot, M.area(side, margin + M.BRAKE_ZONE)) then
							return raw_direct(id, t, point, ...)
						end
					end
				end
			end
			return raw(id, t, point, ...)
		end
	end
end

return M
