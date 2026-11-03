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
M.PATH_STEP = 0.08        -- m between the points of a shot's path raced against an enemy
M.RHO = 0.5               -- V_LOST(c) = -RHO * V(mirror c): how much conceding weighs
M.CELL = 0.25             -- m, grid cell size
M.FIELD_HALF_X, M.FIELD_HALF_Y = 4.5, 3.0  -- m
M.FIELD_MARGIN = 0.25     -- m, a kicked ball must stop at least this far inside the field
M.DIRECTIONS = 32         -- kick directions tried for a kick into space
M.LINE_UP = 0.6           -- s, line-up time assumed in the grid (no robots there)
M.KEEPER_LINE = 0.15      -- m, the modelled keeper stands this far in front of the goal line
M.SWEEPS = 60             -- max value-iteration sweeps
M.GRID_KEEP = 0.8         -- grid only: chance a kick into space is ours again (typical defense)
M.KEEPER_DT = 0.03        -- s, time step of the keeper simulation
M.KEEPER_SPEED = 1.8      -- m/s, top speed of the tracking keeper (profile cruise_speed, measured)
M.KEEPER_LAG = 0.4        -- s, the keeper follows where the ball was this long ago (controller start-up)
M.KEEPER_ACCEL = 6.0      -- m/s^2 (simulator.toml max_accel)
M.KEEPER_GAP_SCALE = 0.04 -- m, spread of P_keeper = σ(gap / scale) around a touch (gap 0)
M.MIN_PASS_DIST = 1.0     -- m, a mate closer than this is not worth passing to
M.BLOCK_DIST = 0.3        -- m, a blocked shot ends up about this far from where it was kicked (3v3 guard: 0.3 m, run 00)
M.BLOCK_TIME = 0.6        -- s, until a blocked shot is played again (kicks into the guard came ~1 s apart)
M.GUARD_AREA_MARGIN = profile.robot.radius + 0.03  -- m, a guard keeps its body this far out of the area (the rule)
M.PUSH_STEP = 0.5         -- m, how far one push carries the ball
M.PUSH_SPEED = 1.0        -- m/s, push speed (skills/push_ball keeps the ball on the face up to 1.5)
M.PUSH_ANGLES = { 0.0, math.pi / 6, -math.pi / 6, math.pi / 3, -math.pi / 3 }  -- rad, off the line to the goal center
M.PUSH_LINE_UP = 0.1      -- s, grid only: line-up of a push after a push (the robot is behind the ball)
M.FOLLOW_DIST = 0.6       -- m, the field enemy nearest the ball within this follows it (backs off when pushed)
M.PUSH_APPROACH = 0.04    -- m, gap between robot and ball at the push's approach point (skills/push_ball)
M.ROLL_ROOM = 0.25        -- m, a pushed ball rolling on must stop this far out of an area (the kick that takes over
                          -- stops pushing mid-way; run 03: 3 cm of room let balls roll in)
M.PUSH_ROOM = 0.05        -- m, the pushing robot stops this much further out of an area than area_guard asks
M.RECEIVER_AIM_STEP = 0.15  -- m between aims tried for a receiver's shot (cheaper, many spots)

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

--- P(the kick leaves before an enemy takes the ball): an enemy that reaches the ball first
--- takes it only as often as the opponent challenges (profile.opponent.challenges_ball).
--- @param ball { x: number, y: number }
--- @param enemies RobotState[]
--- @param t_kick number  -- s until our kick
--- @return number
function M.p_release(ball, enemies, t_kick)
	local challenge = profile.opponent.challenges_ball or 1.0
	return 1.0 - challenge * (1.0 - M.race(tempo.enemy_time(ball, enemies) - t_kick))
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
--- A robot whose body already covers the path is no race: −∞ (blocked). Timing noise is
--- about who gets there first; a robot standing on the line is there (the 3v3 ball guard,
--- 0.35 m in front of the ball, was raced at 39% clear and stopped every shot).
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
			if dist(r, p) <= profile.touch_distance() then return -math.huge end
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
---     S = P_rel·[P_clear·γ^t·(P_on + (1 − P_on)·V_LOST(aim)) + (1 − P_clear)·γ^{t'}·loose(b')]
---         + (1 − P_rel)·γ^{t_kick}·V_LOST(b)
--- P_on = P_target·P_keeper. Missed or saved: the ball is theirs (the keeper holds it, or it
--- goes out). Blocked by a field robot: a loose ball just past the ball (b', loose).
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
	local p_rel = M.p_release(ball, enemies, t_kick)
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
	-- Failures, as for every other kick: missed, blocked or saved -> the ball is theirs near
	-- the aim; taken before the kick -> theirs where it is.
	local p_on = clamp(p_target) * clamp(p_keeper)  -- past the keeper and between the posts
	local d = math.max(dist(ball, aim), 1e-6)
	local blocked_at = {  -- a blocked shot stays near the ball: just past it, towards the aim
		x = ball.x + (aim.x - ball.x) * M.BLOCK_DIST / d,
		y = ball.y + (aim.y - ball.y) * M.BLOCK_DIST / d,
	}
	local score = clamp(p_rel) * (
			clamp(p_clear) * M.discount(t) * (p_on + (1 - p_on) * M.V_LOST(aim))
			+ (1 - clamp(p_clear)) * M.discount(t_kick + M.BLOCK_TIME) * M.loose(blocked_at))
		+ (1 - clamp(p_rel)) * M.discount(t_kick) * M.V_LOST(ball)
	return {
		aim = aim, score = score, p = p, t = t,
		p_rel = p_rel, p_target = p_target, p_clear = p_clear, p_keeper = p_keeper,
	}
end

--- The best shot from where the ball is: the aim along the goal mouth with the highest
--- score. nil when no aim can be reached (the goal is beyond a kick's roll).
--- @param robot RobotState
--- @param ball { x: number, y: number }
--- @param enemies RobotState[]
--- @param aim_step? number  -- m between aims tried (default AIM_STEP)
--- @return table|nil  -- see shot_at
function M.best_shot(robot, ball, enemies, aim_step)
	aim_step = aim_step or M.AIM_STEP
	local keeper, field = M.split_keeper(enemies)
	local inner = M.GOAL_HALF_WIDTH - profile.ball.radius
	-- Coarse to fine: every second aim first, then the two next to the best one.
	local steps = math.floor(inner / aim_step + 1e-9)
	local best, best_i = nil, nil
	local function try(i)
		if i < -steps or i > steps then return end
		local aim = { x = M.GOAL.x, y = M.GOAL.y + i * aim_step }
		local shot = M.shot_at(robot, ball, aim, keeper, field)
		if shot and (not best or shot.score > best.score) then
			best, best_i = shot, i
		end
	end
	for i = -steps, steps, 2 do try(i) end
	if best_i then
		local center = best_i
		try(center - 1)
		try(center + 1)
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

--- The enemies of the grid model for a ball at `ball`: the model keeper and, when the
--- opponent guards the ball (profile.opponent.guards_ball), a guard guard_distance in front
--- of the ball towards the goal.
--- @return RobotState[]
function M.model_enemies(ball)
	local enemies = { M.model_keeper(ball) }
	local guard = profile.opponent.guard_distance
	if guard and profile.opponent.guards_ball then
		local d = math.max(dist(ball, M.GOAL), 1e-3)
		local spot = { x = ball.x + (M.GOAL.x - ball.x) * guard / d, y = ball.y + (M.GOAL.y - ball.y) * guard / d }
		-- A field robot may not enter the defense area: the guard stops at its front edge.
		local front = M.GOAL.x - area_guard.AREA_DEPTH - M.GUARD_AREA_MARGIN
		if spot.x > front and math.abs(spot.y) < area_guard.AREA_HALF_Y + M.GUARD_AREA_MARGIN then
			spot.x = front
		end
		enemies[#enemies + 1] = {
			x = spot.x, y = spot.y,
			orientation = math.atan(ball.y - M.GOAL.y, ball.x - M.GOAL.x), vel_x = 0, vel_y = 0,
		}
	end
	return enemies
end

--- Where a kick from `ball` in direction `angle` stops (a kick's roll), or nil when it
--- would leave the field, stop in a defense area or roll through the enemy's.
--- @return { x: number, y: number }|nil
function M.landing(ball, angle)
	local r = profile.roll_distance()
	local p = { x = ball.x + r * math.cos(angle), y = ball.y + r * math.sin(angle) }
	if math.abs(p.x) > M.FIELD_HALF_X - M.FIELD_MARGIN or math.abs(p.y) > M.FIELD_HALF_Y - M.FIELD_MARGIN then
		return nil
	end
	-- A ball rolling through the enemy area is the keeper's.
	if area_guard.touched(p, profile.ball.radius) or area_guard.segment_crosses(ball, p, 1, profile.ball.radius) then
		return nil
	end
	return p
end

local grid = nil  -- { nx, ny, v[i][j] }, built on first use
local building = false  -- true while the grid is built (V_LOST is not known yet)

local function cell_center(i, j)
	return { x = -M.FIELD_HALF_X + (i - 0.5) * M.CELL, y = -M.FIELD_HALF_Y + (j - 0.5) * M.CELL }
end

--- Builds V by value iteration on the empty field (only the model keeper and guard):
---     V(c) = max( shot(c), max_dir γ^{LINE_UP + flight} · [q·V(p) + (1 − q)·V_LOST(p)],
---                 max_push γ^{PUSH_LINE_UP + STEP/PUSH_SPEED} · [k·V(c') + (1 − k)·V_LOST(c')] )
--- with p = landing(c, dir), q = GRID_KEEP standing in for the enemies the grid leaves out,
--- c' a push target (push_targets) and k = 1 − challenges_ball.
--- Cells inside a defense area are 0 (no field robot may play the ball there).
local function build()
	building = true
	local nx = math.floor(2 * M.FIELD_HALF_X / M.CELL + 0.5)
	local ny = math.floor(2 * M.FIELD_HALF_Y / M.CELL + 0.5)
	local v, shot, moves, pushes = {}, {}, {}, {}
	local roll = profile.roll_distance()
	local t_move = M.LINE_UP + profile.ball_travel_time(0.95 * roll)
	for i = 1, nx do
		v[i], shot[i], moves[i], pushes[i] = {}, {}, {}, {}
		for j = 1, ny do
			local c = cell_center(i, j)
			local blocked = area_guard.touched(c, profile.ball.radius) ~= nil
			local s = 0.0
			if not blocked then
				local robot = { x = c.x - 0.15, y = c.y, orientation = 0.0 }
				local best = M.best_shot(robot, c, M.model_enemies(c))
				s = best and best.score or 0.0
			end
			shot[i][j], v[i][j] = s, s
			moves[i][j] = {}
			pushes[i][j] = {}
			if not blocked then
				for _, q in ipairs(M.push_targets(c, M.model_enemies(c))) do
					local qi = math.max(1, math.min(nx, math.floor((q.x + M.FIELD_HALF_X) / M.CELL) + 1))
					local qj = math.max(1, math.min(ny, math.floor((q.y + M.FIELD_HALF_Y) / M.CELL) + 1))
					pushes[i][j][#pushes[i][j] + 1] = { qi, qj }
				end
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
	local push_discount = M.discount(M.PUSH_LINE_UP + M.PUSH_STEP / M.PUSH_SPEED)
	local push_keep = 1.0 - (profile.opponent.challenges_ball or 1.0)
	for _ = 1, M.SWEEPS do
		local change = 0.0
		for i = 1, nx do
			for j = 1, ny do
				local best = shot[i][j]
				for _, m in ipairs(moves[i][j]) do
					local lost = -M.RHO * v[nx + 1 - m[1]][ny + 1 - m[2]]
					best = math.max(best, discount * (M.GRID_KEEP * v[m[1]][m[2]] + (1 - M.GRID_KEEP) * lost))
				end
				for _, m in ipairs(pushes[i][j]) do
					local lost = -M.RHO * v[nx + 1 - m[1]][ny + 1 - m[2]]
					best = math.max(best, push_discount * (push_keep * v[m[1]][m[2]] + (1 - push_keep) * lost))
				end
				change = math.max(change, math.abs(best - v[i][j]))
				v[i][j] = best
			end
		end
		if change < 1e-4 then break end
	end
	grid = { nx = nx, ny = ny, v = v }
	building = false
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
	if building then return 0.0 end  -- the grid's own shots: success only
	return -M.RHO * M.V({ x = -p.x, y = -p.y })
end

--- Value of a loose ball at `p` that an enemy got to first (a blocked shot, a cut pass):
--- it only takes it as often as the opponent challenges (profile.opponent.challenges_ball);
--- otherwise the ball stays ours there.
---     loose(p) = c·V_LOST(p) + (1 − c)·V(p)
--- @param p { x: number, y: number }
--- @return number
function M.loose(p)
	if building then return 0.0 end
	local c = profile.opponent.challenges_ball or 1.0
	return c * M.V_LOST(p) + (1 - c) * M.V(p)
end

-- ---------------------------------------------------------------------------
-- kick into space (section 3.4)
-- ---------------------------------------------------------------------------

--- Scores kicking the ball from `ball` into space at `p` (where the kick stops):
---     S = P_rel·γ^t·[P_win·V(p) + (1 − P_win)·loose(p)] + (1 − P_rel)·γ^{t_kick}·V_LOST(b)
--- P_win = race(tempo.loose_ball_margin): we reach the rolling ball before they do.
--- @param robot RobotState  -- the kicker
--- @param ball { x: number, y: number }
--- @param p { x: number, y: number }
--- @param allies RobotState[]  -- the kicker's teammates
--- @param enemies RobotState[]
--- @return table  -- { target, score, p_rel, p_win, t }
function M.kick_into_space(robot, ball, p, allies, enemies)
	local t_kick = tempo.line_up_time(robot, ball, p)
	local p_rel = M.p_release(ball, enemies, t_kick)
	local p_win = M.race(tempo.loose_ball_margin(robot, ball, p, allies, enemies))
	local t = t_kick + profile.ball_travel_time(0.95 * dist(ball, p))
	local score = p_rel * M.discount(t) * (p_win * M.V(p) + (1 - p_win) * M.loose(p))
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

-- ---------------------------------------------------------------------------
-- push (section 3.3, the dribble)
-- ---------------------------------------------------------------------------

--- Whether an enemy body stands on the push from `ball` to `q`. The field enemy nearest
--- the ball is taken as its follower: it backs off to its guard spot in front of q (kept
--- out of the defense area like model_enemies' guard), so it only blocks when that spot
--- touches the ball at q. The others block where they stand.
local function push_blocked(ball, q, enemies)
	local keeper, field = M.split_keeper(enemies)
	local follower, best = nil, M.FOLLOW_DIST
	for _, e in ipairs(field) do
		local d = dist(e, ball)
		if d < best then follower, best = e, d end
	end
	local reach = profile.touch_distance()
	for _, e in ipairs(enemies) do
		if e == follower then
			local model = M.model_enemies(q)
			local spot = model[2]
			if spot and dist(spot, q) <= reach then return true end
		else
			local steps = math.max(1, math.ceil(dist(ball, q) / M.PATH_STEP))
			for i = 1, steps do
				local k = i / steps
				local p = { x = ball.x + (q.x - ball.x) * k, y = ball.y + (q.y - ball.y) * k }
				if dist(e, p) <= reach then return true end
			end
		end
	end
	return false
end

--- Where pushes from `ball` can take it: PUSH_STEP along each of PUSH_ANGLES off the line
--- to the goal center, keeping the pushing robot (behind the ball) out of the defense areas
--- with room to stop, the ball inside the field, and no enemy body on the way.
--- @param ball { x: number, y: number }
--- @param enemies RobotState[]
--- @return { x: number, y: number }[]
function M.push_targets(ball, enemies)
	local out = {}
	local base = math.atan(M.GOAL.y - ball.y, M.GOAL.x - ball.x)
	local c = profile.contact_distance()
	for _, a in ipairs(M.PUSH_ANGLES) do
		local ux, uy = math.cos(base + a), math.sin(base + a)
		local q = { x = ball.x + ux * M.PUSH_STEP, y = ball.y + uy * M.PUSH_STEP }
		local robot_end = { x = q.x - ux * c, y = q.y - uy * c }
		local robot_start = { x = ball.x - ux * c, y = ball.y - uy * c }
		-- The approach spot behind the ball: the path planner (move_to) cannot reach a point
		-- outside the field lines.
		local back = profile.touch_distance() + M.PUSH_APPROACH
		local approach = { x = ball.x - ux * back, y = ball.y - uy * back }
		local margin = area_guard.MARGIN + M.PUSH_ROOM
		if math.abs(q.x) <= M.FIELD_HALF_X - M.FIELD_MARGIN and math.abs(q.y) <= M.FIELD_HALF_Y - M.FIELD_MARGIN
			and math.abs(approach.x) < M.FIELD_HALF_X and math.abs(approach.y) < M.FIELD_HALF_Y
			and not area_guard.touched(robot_start, margin) and not area_guard.touched(robot_end, margin)
			and not area_guard.segment_crosses(robot_start, robot_end, 1, margin)
			and not area_guard.segment_crosses(robot_start, robot_end, -1, margin)
			and not push_blocked(ball, q, enemies) then
			out[#out + 1] = q
		end
	end
	return out
end

--- Whether a ball pushed from `ball` to `q` may roll on freely once the push ends (it rolls
--- PUSH_SPEED² / (2·roll_decel) further): it stays in the field and out of the defense areas.
--- Otherwise the push has to brake so the ball stops at q (skills/push_ball `brake`).
--- @return boolean
function M.push_rolls_on(ball, q)
	local d = math.max(dist(ball, q), 1e-6)
	local roll = M.PUSH_SPEED ^ 2 / (2 * profile.ball.roll_decel)
	local stop = { x = q.x + (q.x - ball.x) / d * roll, y = q.y + (q.y - ball.y) / d * roll }
	local r = profile.ball.radius + M.ROLL_ROOM
	return math.abs(stop.x) <= M.FIELD_HALF_X - M.FIELD_MARGIN and math.abs(stop.y) <= M.FIELD_HALF_Y - M.FIELD_MARGIN
		and not area_guard.touched(stop, r)
		and not area_guard.segment_crosses(q, stop, 1, r) and not area_guard.segment_crosses(q, stop, -1, r)
end

--- Scores pushing the ball from `ball` to `q` (section 3.3):
---     S = γ^t·[P_keep·V(q) + (1 − P_keep)·V_LOST(q)],  t = t_line_up + |q − b| / PUSH_SPEED
--- P_keep = P_release over the whole push: an enemy that reaches the ball before the push
--- ends takes it as often as the opponent challenges.
--- @return table  -- { target, score, p_keep, t }
function M.push_to(robot, ball, q, enemies)
	local t = tempo.line_up_time(robot, ball, q) + dist(ball, q) / M.PUSH_SPEED
	local p_keep = M.p_release(ball, enemies, t)
	local score = M.discount(t) * (p_keep * M.V(q) + (1 - p_keep) * M.V_LOST(q))
	return { target = q, score = score, p_keep = p_keep, t = t }
end

--- The best push from `ball`, or nil when no push target is free.
--- @return table|nil  -- see push_to
function M.best_push(robot, ball, enemies)
	local best = nil
	for _, q in ipairs(M.push_targets(ball, enemies)) do
		local option = M.push_to(robot, ball, q, enemies)
		if not best or option.score > best.score then best = option end
	end
	return best
end

-- ---------------------------------------------------------------------------
-- pass (section 3.2)
-- ---------------------------------------------------------------------------

--- The enemies when a pass to `p` has been received `t` s from now: the field enemy
--- nearest the ball follows it to its guard spot in front of p (as far as it gets in t);
--- the keeper and the other enemies stay.
--- @param enemies RobotState[]
--- @param ball { x: number, y: number }
--- @param p { x: number, y: number }
--- @param t number
--- @return RobotState[]
function M.enemies_when_received(enemies, ball, p, t)
	local keeper, field = M.split_keeper(enemies)
	local follower, best = nil, math.huge
	for _, e in ipairs(field) do
		local d = dist(e, ball)
		if d < best then follower, best = e, d end
	end
	local out = {}
	for _, e in ipairs(enemies) do
		if e == follower then
			out[#out + 1] = tempo.enemies_at({ e }, p, t)[1]
		else
			out[#out + 1] = e
		end
	end
	return out
end

--- Scores a pass from `ball` to the teammate `mate` where it stands:
---     S = P_rel·[P_lane·P_ctl·γ^{t_r}·W + P_lane·(1 − P_ctl)·γ^{t_a}·loose(p)
---              + (1 − P_lane)·γ^{t_m}·loose(mid)] + (1 − P_rel)·γ^{t_kick}·V_LOST(b)
--- W = max(V(p), the mate's best shot from p): what holding the ball there is worth.
--- nil when the mate is too close, out of a kick's roll, or the kick cannot be taken.
--- @param passer RobotState
--- @param ball { x: number, y: number }
--- @param mate RobotState
--- @param enemies RobotState[]
--- @param t_ready? number  -- s until the mate stands there (default 0: it is there)
--- @return table|nil  -- { receiver, score, p_rel, p_lane, p_ctl, w }
function M.pass_to(passer, ball, mate, enemies, t_ready)
	local p = { x = mate.x, y = mate.y }
	local d = dist(ball, p)
	if d < M.MIN_PASS_DIST or not M.kickable(ball, p) then return nil end
	local flight = profile.ball_travel_time(d)
	if flight == math.huge then return nil end

	local t_kick = math.max(tempo.line_up_time(passer, ball, p), t_ready or 0.0)
	local p_rel = M.p_release(ball, enemies, t_kick)
	local react = tempo.reaction(t_kick)
	local later = tempo.enemies_at(enemies, ball, react)
	local p_lane = M.race(M.path_margin(ball, p, t_kick, later, react))
	local t_arrive = t_kick + flight
	local t_recv = tempo.receive_time(p, ball, M.GOAL)
	local p_ctl = M.race(react + tempo.enemy_time(p, later) - t_arrive - t_recv)

	-- The mate after receiving: at p, facing the ball. The enemy following the ball (the
	-- one nearest it) heads for its guard spot in front of p meanwhile (tempo.enemies_at:
	-- profile.opponent.guard_distance); the keeper and the others stay where they are.
	local receiver = { x = p.x, y = p.y, orientation = math.atan(ball.y - p.y, ball.x - p.x) }
	local at_shot = M.enemies_when_received(enemies, ball, p, t_arrive + t_recv)
	local shot = M.best_shot(receiver, p, at_shot, M.RECEIVER_AIM_STEP)
	local w = math.max(M.V(p), shot and shot.score or 0.0)
	local mid = { x = (ball.x + p.x) / 2, y = (ball.y + p.y) / 2 }

	local score = p_rel * (p_lane * p_ctl * M.discount(t_arrive + t_recv) * w
			+ p_lane * (1 - p_ctl) * M.discount(t_arrive) * M.loose(p)
			+ (1 - p_lane) * M.discount(t_kick + flight / 2) * M.loose(mid))
		+ (1 - p_rel) * M.discount(t_kick) * M.V_LOST(ball)
	return { receiver = mate.id, score = score, p_rel = p_rel, p_lane = p_lane, p_ctl = p_ctl, w = w }
end

return M
