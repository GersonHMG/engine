--- tempo.lua -- who gets there first: our kicks against the enemies' runs.
---
--- Pure module: every input is passed in; times come from the robot profile
--- (utils/robot_profile), so a profile measured on real robots changes them.
---
--- A play is a race. Ours: line up the kick, the ball's flight, the receiver's
--- trap and turn (none for a one-touch shot), then the shot's flight. Theirs:
--- the nearest enemy running to a point of the ball's path, from where it is
--- now and counting how it is moving (a defender charging one way needs to
--- brake before covering the other). The margin, in seconds, is how much
--- earlier the ball passes than the enemy arrives; profile.first_chance turns
--- it into a probability.
---
---     margin(path) = min over points p of the path, enemies e:  react + T_e(p) − T_ball(p)
---
--- `react` is when the enemies start running for this path. Defenders that follow
--- the ball only start when the pass is kicked: until then they keep shadowing the
--- passer, and that is what a 2v1 exploits. profile.opponent.read_passes (0..1) says
--- how much of our line-up time an opponent uses to read the pass in advance, and
--- profile.opponent.guard_distance where a ball-following opponent goes meanwhile
--- (that far in front of the ball, towards its goal): the race after the kick starts
--- from there (M.enemies_at), not from where the enemy stands now.

local profile = require("utils.robot_profile")

local M = {}

M.ORBIT_CLEARANCE = 0.0885  -- m, gap between robot and ball while circling it (orbit_ball)
M.SAMPLE_STEP = 0.3         -- m between the points of a path where an enemy could cut it
M.GOAL = { x = 4.5, y = 0.0 }
M.GOAL_HALF_WIDTH = 0.5     -- m, SSL division B goal (1.0 m wide)
M.AIM_STEP = 0.1            -- m between the aim points tried

local function wrap(angle)
	while angle > math.pi do angle = angle - 2 * math.pi end
	while angle <= -math.pi do angle = angle + 2 * math.pi end
	return angle
end

local function dist(a, b)
	return math.sqrt((a.x - b.x) ^ 2 + (a.y - b.y) ^ 2)
end

local function angle_to(from, to)
	return math.atan(to.y - from.y, to.x - from.x)
end

--- Seconds for `robot` to line up behind `ball` and kick it towards `target`: drive to
--- the circle around the ball, around it to the spot behind it, and face the target.
--- @param robot { x: number, y: number, orientation: number }
--- @param ball { x: number, y: number }
--- @param target { x: number, y: number }
--- @return number
function M.line_up_time(robot, ball, target)
	local motion = profile.robot
	local kick = angle_to(ball, target)
	local radius = profile.touch_distance() + M.ORBIT_CLEARANCE
	local around = math.abs(wrap(kick + math.pi - angle_to(ball, robot)))
	local reach = math.max(0.0, dist(robot, ball) - radius)
	local turn = math.abs(wrap(robot.orientation - kick))
	return motion.move_overhead + (reach + around * radius) / motion.cruise_speed + turn / motion.turn_rate
end

--- Angle between where the ball comes from and the aim, seen from the receiver at `at`.
--- @return number  -- rad, in [0, pi]
function M.redirect_angle(at, from, aim)
	return math.abs(wrap(angle_to(at, aim) - angle_to(at, from)))
end

--- Whether a ball passed from `from` can be shot at `aim` on the first touch from `at`.
function M.one_touch(at, from, aim)
	return M.redirect_angle(at, from, aim) <= profile.kick.one_touch_angle
end

--- Seconds from the ball reaching a receiver at `at` (passed from `from`) to its shot at
--- `aim`: none for a one-touch shot; otherwise it traps the ball facing the pass, turned
--- towards the aim as far as the kicker face allows, and circles the ball for the rest.
--- @return number
function M.receive_time(at, from, aim)
	if M.one_touch(at, from, aim) then
		return 0.0
	end
	local facing = angle_to(at, from)
	local pre_turn = math.min(M.redirect_angle(at, from, aim), profile.kicker_acceptance())
	local side = wrap(angle_to(at, aim) - facing) >= 0 and 1 or -1
	local robot = { x = at.x, y = at.y, orientation = facing + side * pre_turn }
	local c = profile.contact_distance()
	local ball = { x = at.x + c * math.cos(facing), y = at.y + c * math.sin(facing) }
	return M.line_up_time(robot, ball, aim)
end

--- Seconds the fastest enemy needs to get its body onto `point` (within touch distance).
--- @param point { x: number, y: number }
--- @param enemies RobotState[]
--- @return number
function M.enemy_time(point, enemies)
	local best = math.huge
	local reach = profile.touch_distance()
	for _, e in ipairs(enemies) do
		local d = dist(e, point)
		local p = point
		if d > reach then
			p = { x = point.x + (e.x - point.x) * reach / d, y = point.y + (e.y - point.y) * reach / d }
		end
		best = math.min(best, d <= reach and 0.0 or profile.enemy_time_to_reach(e, p))
	end
	return best
end

--- Where the enemies will be at `t` s from now while we line up a kick of `ball`: a
--- ball-following opponent (profile.opponent.guard_distance set) runs to its guard spot,
--- guard_distance in front of the ball towards the goal, as far as it gets in `t` and
--- arriving at rest; otherwise it stays where it is. The race for the kick starts there.
--- @param enemies RobotState[]
--- @param ball { x: number, y: number }
--- @param t number
--- @return RobotState[]
function M.enemies_at(enemies, ball, t)
	local guard = profile.opponent.guard_distance
	if not guard or t <= 0 then
		return enemies
	end
	local motion = profile.opponent
	local length = math.max(dist(ball, M.GOAL), 1e-3)
	local spot = { x = ball.x + (M.GOAL.x - ball.x) * guard / length, y = ball.y + (M.GOAL.y - ball.y) * guard / length }
	local moved = {}
	for i, e in ipairs(enemies) do
		local d = dist(e, spot)
		local reach = math.max(0.0, t - profile.enemy_time_to_reach(e, spot) + d / motion.cruise_speed) * motion.cruise_speed
		local k = d > 1e-6 and math.min(1.0, reach / d) or 1.0
		moved[i] = {
			id = e.id, team = e.team, active = e.active,
			x = e.x + (spot.x - e.x) * k, y = e.y + (spot.y - e.y) * k,
			orientation = math.atan(ball.y - spot.y, ball.x - spot.x),
			vel_x = 0.0, vel_y = 0.0,
		}
	end
	return moved
end

--- When enemies start running for a path that is decided at `t_kick` (s from now).
--- @param t_kick number
--- @return number
function M.reaction(t_kick)
	return (1.0 - profile.opponent.read_passes) * t_kick
end

--- Margin, s, of a kick from `from` to `to` leaving at time `t0`: min over the points of
--- the path of (enemy arrival − ball arrival), the enemies starting to run at `react`
--- (default 0: now). Negative: an enemy cuts it. With `ball_point`, the kick point itself
--- counts too, raced from now (an enemy reaching the ball before t0 takes it).
--- @param from { x: number, y: number }
--- @param to { x: number, y: number }
--- @param t0 number
--- @param enemies RobotState[]
--- @param react? number
--- @param ball_point? boolean
--- @return number
function M.path_margin(from, to, t0, enemies, react, ball_point)
	react = react or 0.0
	local length = dist(from, to)
	local steps = math.max(1, math.ceil(length / M.SAMPLE_STEP))
	local worst = ball_point and (M.enemy_time(from, enemies) - t0) or math.huge
	for i = 1, steps do
		local s = length * i / steps
		local p = { x = from.x + (to.x - from.x) * i / steps, y = from.y + (to.y - from.y) * i / steps }
		local ball = t0 + profile.ball_travel_time(s)
		worst = math.min(worst, react + M.enemy_time(p, enemies) - ball)
	end
	return worst
end

--- How far from the goal center a shot from `from` may aim and still go in with the
--- profile's typical kick direction error (kick.angle_error), m; 0 when only the center is safe.
function M.aim_half_width(from)
	local spread = dist(from, M.GOAL) * math.tan(profile.kick.angle_error)
	return math.max(0.0, M.GOAL_HALF_WIDTH - profile.ball.radius - spread)
end

--- The aim point in the goal mouth with the largest margin for a shot from `from` leaving
--- at `t0`, and that margin. Shots away from the defender's side (the far post after a
--- switch) win here, within the width the kick's accuracy allows (aim_half_width).
--- @return { x: number, y: number } aim
--- @return number margin
function M.best_aim(from, t0, enemies, react)
	local best, best_margin = M.GOAL, -math.huge
	local steps = math.floor(M.aim_half_width(from) / M.AIM_STEP + 1e-9)
	for i = -steps, steps do
		local aim = { x = M.GOAL.x, y = M.GOAL.y + i * M.AIM_STEP }
		-- Ties go to the aim nearer the center.
		local margin = M.path_margin(from, aim, t0, enemies, react) - 1e-6 * math.abs(i)
		if margin > best_margin then
			best, best_margin = aim, margin
		end
	end
	return best, best_margin
end

--- Race for a loose ball kicked by `kicker` from `ball` towards `target` (a clear, a kick
--- into space): margin, s, of our first touch before theirs. Anyone touches the ball at the
--- first point of its roll they reach no later than the ball (the kicker starts after its
--- kick, from the ball; the enemies after `reaction`, from where they will be then).
--- @param kicker RobotState
--- @param ball { x: number, y: number }
--- @param target { x: number, y: number }  -- where the ball stops
--- @param allies RobotState[]  -- the kicker's teammates
--- @param enemies RobotState[]
--- @return number
function M.loose_ball_margin(kicker, ball, target, allies, enemies)
	local t_kick = M.line_up_time(kicker, ball, target)
	local react = M.reaction(t_kick)
	local later = M.enemies_at(enemies, ball, react)
	local from_ball = { x = ball.x, y = ball.y, orientation = angle_to(ball, target), vel_x = 0, vel_y = 0 }
	local length = dist(ball, target)
	local steps = math.max(1, math.ceil(length / M.SAMPLE_STEP))
	local ours, theirs = math.huge, M.enemy_time(ball, enemies) -- they may take it before the kick
	for i = 1, steps do
		local p = { x = ball.x + (target.x - ball.x) * i / steps, y = ball.y + (target.y - ball.y) * i / steps }
		local t_ball = t_kick + profile.ball_travel_time(length * i / steps)
		local mine = t_kick + profile.time_to_reach(from_ball, p)
		for _, a in ipairs(allies) do
			mine = math.min(mine, profile.time_to_reach(a, p))
		end
		ours = math.min(ours, math.max(t_ball, mine))
		theirs = math.min(theirs, math.max(t_ball, react + M.enemy_time(p, later)))
	end
	return theirs - ours
end

--- Margin, s, of passing from `passer` (kicking `ball`) to a receiver at `at` who then
--- shoots: the worst of the pass (intercepted, or the ball taken before the kick) and the
--- shot (blocked before it reaches the goal), and the time the shot leaves. With the goal
--- beyond a kick's roll from `at`, there is no shot yet: the second part is the receiver
--- getting the ball under control (trapped and turned) before an enemy reaches it.
--- @param passer RobotState
--- @param ball { x: number, y: number }
--- @param at { x: number, y: number }
--- @param enemies RobotState[]
--- @param t_ready? number  -- s until the receiver is at `at` (default 0: it is there)
--- @return number margin
--- @return number shot_time  -- s from now until the receiver's shot leaves
function M.pass_margin(passer, ball, at, enemies, t_ready)
	local t_kick = math.max(M.line_up_time(passer, ball, at), t_ready or 0.0)
	local flight = profile.ball_travel_time(dist(ball, at))
	if flight == math.huge then
		return -math.huge, math.huge
	end
	local react = M.reaction(t_kick)
	-- The ball itself is raced from now; the rest from where the enemies are at `react`.
	local ball_margin = M.enemy_time(ball, enemies) - t_kick
	local later = M.enemies_at(enemies, ball, react)
	local pass = math.min(ball_margin, M.path_margin(ball, at, t_kick, later, react))
	local t_shot = t_kick + flight + M.receive_time(at, ball, M.GOAL)
	if dist(at, M.GOAL) > profile.roll_distance() then
		return math.min(pass, react + M.enemy_time(at, later) - t_shot), t_shot
	end
	local _, shot = M.best_aim(at, t_shot, later, react)
	return math.min(pass, shot), t_shot
end

return M
