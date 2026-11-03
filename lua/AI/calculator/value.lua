--- value.lua -- action scores in one unit: the probability that the action ends in a goal.
---
---     score(a, s) = Σ_i p_i · γ^{t_i} · V(s'_i) − cost(a)        (docs/action-scoring.md)
---
--- Success probabilities are products of stages; most stages are races between the ball
--- (or our robot) and the fastest enemy, turned into probabilities by `race`.
--- So far this module has the shot (section 3.1). The V grid is not built yet, so every
--- failure outcome is worth V_LOST = 0 (ρ = 0 in the doc).
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
	-- 4. beats the keeper: it tracks the ball while we line up and moves for the shot
	--    once it is kicked.
	local p_keeper = 1.0
	if keeper then
		p_keeper = M.race(M.path_margin(ball, aim, t_kick, { keeper }, t_kick))
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

return M
