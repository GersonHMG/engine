--- value.lua -- action scores in one unit: the probability that the action ends in a goal.
---
---     score(a, s) = Σ_i p_i · γ^{t_i} · V(s'_i) − cost(a)        (docs/action-scoring.md)
---
--- Success probabilities are products of stages; most stages are races between the ball
--- (or our robot) and the fastest enemy, turned into probabilities by `race`.
--- It has the shot (section 3.1), the state values V / V_LOST on a grid (section 2) and
--- the kick into space (section 3.4).
---
--- Pure module: every input is passed in.

local profile = require("utils.robot_profile")
local tempo = require("AI.calculator.tempo")
local area_guard = require("utils.area_guard")

local M = {}

M.GAMMA = 0.95            -- discount per second: scoring now beats scoring later
M.P_MIN, M.P_MAX = 0.01, 0.99  -- every stage is clamped to this range
M.GOAL = tempo.GOAL       -- enemy goal center
M.GOAL_HALF_WIDTH = tempo.GOAL_HALF_WIDTH
M.AIM_STEP = 0.05         -- m between the aim points tried along the goal mouth
M.KEEPER_RADIUS = 1.5     -- m, the enemy nearest the goal center within this is the keeper
M.PHI_SCALE = 1.702       -- logistic approximation of the normal CDF: Φ(x) ≈ σ(1.702 x)
M.PATH_STEP = 0.05        -- m between the points of a shot's path raced against an enemy
M.RHO = 0.5               -- V_LOST(c) = -RHO * V(mirror c): how much conceding weighs
M.CELL = 0.25             -- m, grid cell size
M.FIELD_HALF_X, M.FIELD_HALF_Y = 4.5, 3.0  -- m
M.FIELD_MARGIN = 0.25     -- m, a kicked ball must stop at least this far inside the field
M.DIRECTIONS = 32         -- kick directions tried for a kick into space
M.LINE_UP = 0.6           -- s, line-up time assumed in the grid (no robots there)
M.KEEPER_LINE = 0.15      -- m, the modelled keeper stands this far in front of the goal line
M.SWEEPS = 60             -- max value-iteration sweeps
M.GRID_KEEP = 0.8         -- grid only: chance a kick into space is ours again (typical defense)
M.KEEPER_DT = 0.02        -- s, time step of the keeper simulation
M.KEEPER_SPEED = 1.8      -- m/s, top speed of the tracking keeper (profile cruise_speed, measured)
M.KEEPER_LAG = 0.4        -- s, the keeper follows where the ball was this long ago (controller start-up)
M.KEEPER_ACCEL = 6.0      -- m/s^2 (simulator.toml max_accel)
M.KEEPER_GAP_SCALE = 0.04 -- m, spread of P_keeper = σ(gap / scale) around a touch (gap 0)

local function dist(a, b)
	return math.sqrt((a.x - b.x) ^ 2 + (a.y - b.y) ^ 2)
end

local function clamp(p)
	return math.max(M.P_MIN, math.min(M.P_MAX, p))
end

local function phi(x)
	return 1.0 / (1.0 + math.exp(-M.PHI_SCALE * x))
end

local function wrap(angle)
	while angle > math.pi do angle = angle - 2 * math.pi end
	while angle <= -math.pi do angle = angle + 2 * math.pi end
	return angle
end

--- P(we get there first) for a race won by `margin` seconds (profile timing noise).
--- @param margin number
--- @return number
function M.race(margin)
	if margin == math.huge then return 1.0 end
	if margin == -math.huge then return 0.0 end
	return profile.first_chance(margin)
end

--- γ^t.
--- @param t number  -- s
--- @return number
function M.discount(t)
	return M.GAMMA ^ t
end

--- The enemy keeper (nearest the goal center, within KEEPER_RADIUS) and the others.
--- @param enemies RobotState[]
--- @return RobotState|nil keeper
--- @return RobotState[] field
function M.split_keeper(enemies)
	local keeper, best = nil, M.KEEPER_RADIUS
	for _, e in ipairs(enemies) do
		local d = dist(e, M.GOAL)
		if d < best then keeper, best = e, d end
	end
	local field = {}
	for _, e in ipairs(enemies) do
		if e ~= keeper then field[#field + 1] = e end
	end
	return keeper, field
end

--- P(on target): a kick aimed at `aim` from `from`, with a normal direction error of
--- kick.angle_error, passes between the posts (the ball's radius inside each post).
--- @param from { x: number, y: number }
--- @param aim { x: number, y: number }
--- @return number
function M.p_on_target(from, aim)
	local inner = M.GOAL_HALF_WIDTH - profile.ball.radius
	local heading = math.atan(aim.y - from.y, aim.x - from.x)
	local low = wrap(math.atan(M.GOAL.y - inner - from.y, M.GOAL.x - from.x) - heading)
	local high = wrap(math.atan(M.GOAL.y + inner - from.y, M.GOAL.x - from.x) - heading)
	local lo, hi = math.min(low, high), math.max(low, high)
	local sigma = profile.kick.angle_error
	return phi(hi / sigma) - phi(lo / sigma)
end

--- Seconds `enemy` needs to get its body onto `point` (within touch distance), moving
--- sideways if it has to: SSL robots are holonomic, so no turn is counted.
local function body_time(enemy, point)
	local reach = profile.touch_distance()
	local d = dist(enemy, point)
	if d <= reach then return 0.0 end
	local motion = profile.opponent
	return (motion.move_overhead or 0) + (d - reach) / motion.cruise_speed
end

--- Margin, s, of a ball kicked from `from` at `t0` along to `to` against `robots` that
--- start moving at `react`: min over the path of (robot arrival − ball arrival). Sampled
--- every PATH_STEP, so a robot standing between two samples is not missed.
--- @return number
function M.path_margin(from, to, t0, robots, react)
	local length = dist(from, to)
	local steps = math.max(1, math.ceil(length / M.PATH_STEP))
	local worst = math.huge
	for i = 1, steps do
		local k = i / steps
		local p = { x = from.x + (to.x - from.x) * k, y = from.y + (to.y - from.y) * k }
		local t_ball = t0 + profile.ball_travel_time(length * k)
		for _, r in ipairs(robots) do
			worst = math.min(worst, react + body_time(r, p) - t_ball)
		end
	end
	return worst
end

--- Gap, m, of a shot against a keeper that tracks the ball (skills/goalkeeper and
--- keepers like it): it does not read the shot, it keeps moving to the point where the
--- line ball -> goal center crosses its line, at most KEEPER_SPEED / KEEPER_ACCEL. The
--- ball rolls from `ball` to `aim`. Simulated every KEEPER_DT: the smallest gap between
--- the keeper's body and the ball, negative when it touches the ball before the ball
--- crosses the goal line.
--- @param ball { x: number, y: number }
--- @param aim { x: number, y: number }
--- @param keeper RobotState
--- @param t0 number  -- s until the kick
--- @return number
function M.keeper_gap(ball, aim, keeper, t0)
	local length = dist(ball, aim)
	if length < 1e-6 then return -math.huge end
	local ux, uy = (aim.x - ball.x) / length, (aim.y - ball.y) / length
	local reach = profile.touch_distance()
	-- The keeper tracks the resting ball during the line-up: it starts there.
	local k = M.model_keeper(ball)
	local kx, ky, vx, vy = k.x, k.y, 0.0, 0.0
	local gap = math.huge
	local t = 0.0
	local total = profile.ball_travel_time(length)
	if total == math.huge then return -math.huge end
	while t <= total + 1e-9 do
		-- Ball position after t s of flight.
		local s = M.travelled(t)
		local bx, by = ball.x + ux * s, ball.y + uy * s
		gap = math.min(gap, dist({ x = kx, y = ky }, { x = bx, y = by }) - reach)
		if gap < 0 then break end
		-- Keeper: accelerate towards its tracking spot for the ball KEEPER_LAG s ago.
		local s_lag = M.travelled(math.max(0.0, t - M.KEEPER_LAG))
		local spot = M.model_keeper({ x = ball.x + ux * s_lag, y = ball.y + uy * s_lag })
		local dx, dy = spot.x - kx, spot.y - ky
		local d = math.sqrt(dx * dx + dy * dy)
		local want_x, want_y = 0.0, 0.0
		if d > 1e-6 then
			local v = math.min(M.KEEPER_SPEED, math.sqrt(2 * M.KEEPER_ACCEL * d))
			want_x, want_y = dx / d * v, dy / d * v
		end
		local ax, ay = want_x - vx, want_y - vy
		local a = math.sqrt(ax * ax + ay * ay)
		local max_dv = M.KEEPER_ACCEL * M.KEEPER_DT
		if a > max_dv then ax, ay = ax / a * max_dv, ay / a * max_dv end
		vx, vy = vx + ax, vy + ay
		kx, ky = kx + vx * M.KEEPER_DT, ky + vy * M.KEEPER_DT
		t = t + M.KEEPER_DT
	end
	return gap
end

--- Distance, m, a kicked ball has rolled after `t` s (kick speed of the profile).
--- @param t number
--- @return number
function M.travelled(t)
	local v0 = profile.kick.speed
	local v1 = v0 * 5 / 7
	local t_slide = (v0 - v1) / profile.ball.slide_decel
	if t <= t_slide then
		return v0 * t - 0.5 * profile.ball.slide_decel * t * t
	end
	local slide = v0 * t_slide - 0.5 * profile.ball.slide_decel * t_slide * t_slide
	local tr = math.min(t - t_slide, v1 / profile.ball.roll_decel)
	return slide + v1 * tr - 0.5 * profile.ball.roll_decel * tr * tr
end

--- Whether a field robot can kick the ball towards `target` without entering a defense
--- area: the spot it kicks from (behind the ball) keeps its body out (utils/area_guard).
--- @param ball { x: number, y: number }
--- @param target { x: number, y: number }
--- @return boolean
function M.kickable(ball, target)
	local d = math.max(dist(ball, target), 1e-6)
	local c = profile.contact_distance()
	local spot = { x = ball.x - c * (target.x - ball.x) / d, y = ball.y - c * (target.y - ball.y) / d }
	return area_guard.touched(spot, area_guard.MARGIN) == nil
end

--- Scores one shot at `aim` (section 3.1):
---     P = P_rel · P_target · P_clear · P_keeper,   score = γ^{t_kick + flight} · P
--- @param robot RobotState  -- the shooter
--- @param ball { x: number, y: number }
--- @param aim { x: number, y: number }
--- @param keeper RobotState|nil
--- @param field RobotState[]  -- the other enemies
--- @return table|nil  -- { aim, score, p, t, p_rel, p_target, p_clear, p_keeper }, nil when the ball cannot get there
function M.shot_at(robot, ball, aim, keeper, field)
	local flight = profile.ball_travel_time(dist(ball, aim))
	if flight == math.huge or not M.kickable(ball, aim) then
		return nil
	end
	local enemies = { table.unpack(field) }
	enemies[#enemies + 1] = keeper
	local t_kick = tempo.line_up_time(robot, ball, aim)

	-- 1. released: the kick leaves before an enemy reaches the ball.
	local p_rel = M.race(tempo.enemy_time(ball, enemies) - t_kick)
	-- 2. on target.
	local p_target = M.p_on_target(ball, aim)
	-- 3. not blocked by a field enemy: they start reading the shot at `react`, from where
	--    they will be after our line-up.
	local react = tempo.reaction(t_kick)
	local p_clear = 1.0
	if #field > 0 then
		local later = tempo.enemies_at(field, ball, react)
		p_clear = M.race(M.path_margin(ball, aim, t_kick, later, react))
	end
	-- 4. beats the keeper: it tracks the ball (keeper_gap).
	local p_keeper = 1.0
	if keeper then
		local gap = M.keeper_gap(ball, aim, keeper, t_kick)
		p_keeper = gap == -math.huge and 0.0 or 1.0 / (1.0 + math.exp(-gap / M.KEEPER_GAP_SCALE))
	end

	local p = clamp(p_rel) * clamp(p_target) * clamp(p_clear) * clamp(p_keeper)
	local t = t_kick + flight
	return {
		aim = aim, score = M.discount(t) * p, p = p, t = t,
		p_rel = p_rel, p_target = p_target, p_clear = p_clear, p_keeper = p_keeper,
	}
end

--- The best shot from where the ball is: the aim along the goal mouth with the highest
--- score. nil when no aim can be reached (the goal is beyond a kick's roll).
--- @param robot RobotState
--- @param ball { x: number, y: number }
--- @param enemies RobotState[]
--- @return table|nil  -- see shot_at
function M.best_shot(robot, ball, enemies)
	local keeper, field = M.split_keeper(enemies)
	local inner = M.GOAL_HALF_WIDTH - profile.ball.radius
	local steps = math.floor(inner / M.AIM_STEP + 1e-9)
	local best = nil
	for i = -steps, steps do
		local aim = { x = M.GOAL.x, y = M.GOAL.y + i * M.AIM_STEP }
		local shot = M.shot_at(robot, ball, aim, keeper, field)
		if shot and (not best or shot.score > best.score) then
			best = shot
		end
	end
	return best
end

-- ---------------------------------------------------------------------------
-- state values (section 2)
-- ---------------------------------------------------------------------------

--- The keeper of the grid model: on the line ball -> goal center, KEEPER_LINE in front of
--- the goal line, between the posts. It is how keepers usually play (skills/goalkeeper).
--- @param ball { x: number, y: number }
--- @return RobotState
function M.model_keeper(ball)
	local line_x = M.GOAL.x - M.KEEPER_LINE
	local dx = ball.x - M.GOAL.x
	local y = M.GOAL.y
	if math.abs(dx) > 1e-3 then
		y = M.GOAL.y + (ball.y - M.GOAL.y) * (line_x - M.GOAL.x) / dx
	end
	y = math.max(-M.GOAL_HALF_WIDTH, math.min(M.GOAL_HALF_WIDTH, y))
	return { x = line_x, y = y, orientation = math.atan(ball.y - y, ball.x - line_x), vel_x = 0, vel_y = 0 }
end

--- Where a kick from `ball` in direction `angle` stops (a kick's roll), or nil when it
--- would leave the field or stop in a defense area.
--- @return { x: number, y: number }|nil
function M.landing(ball, angle)
	local r = profile.roll_distance()
	local p = { x = ball.x + r * math.cos(angle), y = ball.y + r * math.sin(angle) }
	if math.abs(p.x) > M.FIELD_HALF_X - M.FIELD_MARGIN or math.abs(p.y) > M.FIELD_HALF_Y - M.FIELD_MARGIN then
		return nil
	end
	if area_guard.touched(p, profile.ball.radius) then
		return nil
	end
	return p
end

local grid = nil  -- { nx, ny, v[i][j] }, built on first use

local function cell_center(i, j)
	return { x = -M.FIELD_HALF_X + (i - 0.5) * M.CELL, y = -M.FIELD_HALF_Y + (j - 0.5) * M.CELL }
end

--- Builds V by value iteration on the empty field (only the model keeper):
---     V(c) = max( shot(c), max_dir γ^{LINE_UP + flight} · [q·V(p) + (1 − q)·V_LOST(p)] )
--- with p = landing(c, dir) and q = GRID_KEEP standing in for the enemies the grid leaves out.
--- Cells inside a defense area are 0 (no field robot may play the ball there).
local function build()
	local nx = math.floor(2 * M.FIELD_HALF_X / M.CELL + 0.5)
	local ny = math.floor(2 * M.FIELD_HALF_Y / M.CELL + 0.5)
	local v, shot, moves = {}, {}, {}
	local roll = profile.roll_distance()
	local t_move = M.LINE_UP + profile.ball_travel_time(0.95 * roll)
	for i = 1, nx do
		v[i], shot[i], moves[i] = {}, {}, {}
		for j = 1, ny do
			local c = cell_center(i, j)
			local blocked = area_guard.touched(c, profile.ball.radius) ~= nil
			local s = 0.0
			if not blocked then
				local robot = { x = c.x - 0.15, y = c.y, orientation = 0.0 }
				local best = M.best_shot(robot, c, { M.model_keeper(c) })
				s = best and best.score or 0.0
			end
			shot[i][j], v[i][j] = s, s
			moves[i][j] = {}
			if not blocked then
				for k = 0, M.DIRECTIONS - 1 do
					local p = M.landing(c, 2 * math.pi * k / M.DIRECTIONS)
					if p then
						local pi_ = math.max(1, math.min(nx, math.floor((p.x + M.FIELD_HALF_X) / M.CELL) + 1))
						local pj = math.max(1, math.min(ny, math.floor((p.y + M.FIELD_HALF_Y) / M.CELL) + 1))
						moves[i][j][#moves[i][j] + 1] = { pi_, pj }
					end
				end
			end
		end
	end
	local discount = M.discount(t_move)
	for _ = 1, M.SWEEPS do
		local change = 0.0
		for i = 1, nx do
			for j = 1, ny do
				local best = shot[i][j]
				for _, m in ipairs(moves[i][j]) do
					local lost = -M.RHO * v[nx + 1 - m[1]][ny + 1 - m[2]]
					best = math.max(best, discount * (M.GRID_KEEP * v[m[1]][m[2]] + (1 - M.GRID_KEEP) * lost))
				end
				change = math.max(change, math.abs(best - v[i][j]))
				v[i][j] = best
			end
		end
		if change < 1e-4 then break end
	end
	grid = { nx = nx, ny = ny, v = v }
end

--- V(p): the probability that we score from holding the ball at `p` (grid lookup,
--- bilinear between cell centers).
--- @param p { x: number, y: number }
--- @return number
function M.V(p)
	if not grid then build() end
	local fx = (p.x + M.FIELD_HALF_X) / M.CELL + 0.5
	local fy = (p.y + M.FIELD_HALF_Y) / M.CELL + 0.5
	local i = math.max(1, math.min(grid.nx - 1, math.floor(fx)))
	local j = math.max(1, math.min(grid.ny - 1, math.floor(fy)))
	local ax = math.max(0.0, math.min(1.0, fx - i))
	local ay = math.max(0.0, math.min(1.0, fy - j))
	local v = grid.v
	return (1 - ax) * (1 - ay) * v[i][j] + ax * (1 - ay) * v[i + 1][j]
		+ (1 - ax) * ay * v[i][j + 1] + ax * ay * v[i + 1][j + 1]
end

--- V_LOST(p) = −ρ · V(−p): the enemy holding the ball at `p` is our V seen from their side.
--- @param p { x: number, y: number }
--- @return number
function M.V_LOST(p)
	return -M.RHO * M.V({ x = -p.x, y = -p.y })
end

-- ---------------------------------------------------------------------------
-- kick into space (section 3.4)
-- ---------------------------------------------------------------------------

--- Scores kicking the ball from `ball` into space at `p` (where the kick stops):
---     S = P_rel·γ^t·[P_win·V(p) + (1 − P_win)·V_LOST(p)] + (1 − P_rel)·γ^{t_kick}·V_LOST(b)
--- P_win = race(tempo.loose_ball_margin): we reach the rolling ball before they do.
--- @param robot RobotState  -- the kicker
--- @param ball { x: number, y: number }
--- @param p { x: number, y: number }
--- @param allies RobotState[]  -- the kicker's teammates
--- @param enemies RobotState[]
--- @return table  -- { target, score, p_rel, p_win, t }
function M.kick_into_space(robot, ball, p, allies, enemies)
	local t_kick = tempo.line_up_time(robot, ball, p)
	local p_rel = M.race(tempo.enemy_time(ball, enemies) - t_kick)
	local p_win = M.race(tempo.loose_ball_margin(robot, ball, p, allies, enemies))
	local t = t_kick + profile.ball_travel_time(0.95 * dist(ball, p))
	local score = p_rel * M.discount(t) * (p_win * M.V(p) + (1 - p_win) * M.V_LOST(p))
		+ (1 - p_rel) * M.discount(t_kick) * M.V_LOST(ball)
	return { target = p, score = score, p_rel = p_rel, p_win = p_win, t = t }
end

--- The best kick into space from `ball`, over DIRECTIONS directions, or nil when every
--- direction leaves the field, ends in a defense area or cannot be kicked.
--- @return table|nil  -- see kick_into_space
function M.best_kick_into_space(robot, ball, allies, enemies)
	local best = nil
	for k = 0, M.DIRECTIONS - 1 do
		local p = M.landing(ball, 2 * math.pi * k / M.DIRECTIONS)
		if p and M.kickable(ball, p) then
			local option = M.kick_into_space(robot, ball, p, allies, enemies)
			if not best or option.score > best.score then
				best = option
			end
		end
	end
	return best
end

return M
