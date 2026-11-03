-- shot_map: where a shot beats the keeper, measured against value.best_shot's prediction.
--
-- For every ball position of a grid in front of the yellow goal (outside the defense
-- area): blue 0 stands behind the ball, yellow 1 runs skills/goalkeeper, and blue 0 shoots
-- at the aim value.best_shot picks (kick_to_point). Result per spot: goal, saved (keeper
-- touched it), wide, or short. Prints predicted P and the outcome, then the calibration
-- per predicted-P bucket.
--
--   ./target/release/engine --headless --max-time 4000 lua/tests/shot_map.lua
package.path = "lua/?.lua;" .. package.path

local value = require("AI.calculator.value")
-- Keeper model parameters can be swept from the environment (KEEPER_LAG, KEEPER_SPEED).
value.KEEPER_LAG = tonumber(os.getenv("KEEPER_LAG") or "") or value.KEEPER_LAG
value.KEEPER_SPEED = tonumber(os.getenv("KEEPER_SPEED") or "") or value.KEEPER_SPEED
local goalkeeper = require("skills.goalkeeper")
local kick = require("skills.kick_to_point")

local TIME_LIMIT = 5.0  -- s per shot
local GOAL = { x = 4.5, y = 0.0 }
local KEEPER = 1
local BEHIND = 0.25     -- m, robot start behind the ball, on the shot line
-- OFFSET (rad, environment): the robot starts 0.5 m from the ball, this far around it from
-- the shot line, facing the ball (a realistic approach); 0 = lined up already.
local OFFSET = tonumber(os.getenv("OFFSET") or "") or 0.0

local spots = {}
for x = 1.0, 3.31, 0.3 do
	for y = -2.4, 2.41, 0.4 do
		local p = { x = x, y = y }
		if not (x > 3.3 and math.abs(y) < 1.2) then spots[#spots + 1] = p end
	end
end

local current, start, shot, keeper_touched, kicked, cross_y
local rows = {}

local function begin(i)
	current = i
	local ball = spots[i]
	local k = goalkeeper.spot(ball, GOAL)
	local enemy = { x = k.x, y = k.y, orientation = math.pi, vel_x = 0, vel_y = 0 }
	local robot0 = { x = ball.x - BEHIND, y = ball.y, orientation = 0 }
	shot = value.best_shot(robot0, ball, { enemy })
	local aim = shot and shot.aim or GOAL
	local a = math.atan(aim.y - ball.y, aim.x - ball.x)
	if OFFSET ~= 0 then
		local side = (current % 2 == 0) and 1 or -1
		local b = a + math.pi + side * OFFSET
		local rx, ry = ball.x + 0.5 * math.cos(b), ball.y + 0.5 * math.sin(b)
		grsim.teleport_robot(0, 0, rx, ry, math.atan(ball.y - ry, ball.x - rx))
	else
		grsim.teleport_robot(0, 0, ball.x - BEHIND * math.cos(a), ball.y - BEHIND * math.sin(a), a)
	end
	grsim.teleport_robot(KEEPER, 1, k.x, k.y, math.atan(ball.y - k.y, ball.x - k.x))
	grsim.teleport_ball(ball.x, ball.y)
	sim.events()
	start = sim.time()
	keeper_touched = false
	kicked = false
	cross_y = nil
end

local function finish(result)
	local b = spots[current]
	rows[#rows + 1] = { p = shot and shot.p or 0, goal = result == "goal" }
	print(string.format("x=%5.2f y=%5.2f  P=%.2f (tgt %.2f kp %.2f) aim=%5.2f  cross=%5.2f t=%.2f %s",
		b.x, b.y, shot and shot.p or 0, shot and shot.p_target or 0, shot and shot.p_keeper or 0,
		shot and shot.aim.y or 0, cross_y or 99, sim.time() - start, result))
	if current == #spots then
		local buckets = {}
		for _, r in ipairs(rows) do
			local k = math.min(4, math.floor(r.p * 5))
			buckets[k] = buckets[k] or { n = 0, goals = 0, p = 0 }
			buckets[k].n = buckets[k].n + 1
			buckets[k].p = buckets[k].p + r.p
			if r.goal then buckets[k].goals = buckets[k].goals + 1 end
		end
		for k = 0, 4 do
			local bk = buckets[k]
			if bk then
				print(string.format("CALIB P in [%.1f, %.1f): n=%d predicted=%.2f observed=%.2f",
					k / 5, (k + 1) / 5, bk.n, bk.p / bk.n, bk.goals / bk.n))
			end
		end
		sim.finish(true)
	else
		begin(current + 1)
	end
end

begin(1)

function process()
	local aim = shot and shot.aim or GOAL
	-- Only the first kick counts: afterwards the shooter stays where it is.
	if kicked then
		local r = get_robot_state(0, 0)
		move_direct(0, 0, { x = r.x, y = r.y })
	else
		kick.process(0, 0, aim)
	end
	goalkeeper.process(KEEPER, 1, GOAL)
	for _, e in ipairs(sim.events()) do
		if e.kind == "robot_touched_ball" and e.team == 1 then keeper_touched = true end
		if e.kind == "kick" and e.team == 0 then kicked = true end
	end
	local ball = get_ball_state()
	-- Where the ball's straight path crosses the goal line, 0.1 s after the kick.
	if kicked and not cross_y and math.abs(ball.vel_x) > 0.5 then
		cross_y = ball.y + (GOAL.x - ball.x) * ball.vel_y / ball.vel_x
	end
	if ball.x > GOAL.x then
		return finish(math.abs(ball.y) < 0.5 and "goal" or "wide")
	end
	if math.abs(ball.y) > 3.0 then return finish("wide") end
	local speed = math.sqrt(ball.vel_x ^ 2 + ball.vel_y ^ 2)
	if keeper_touched and speed < 0.3 then return finish("saved") end
	if sim.time() - start > TIME_LIMIT then
		return finish(keeper_touched and "saved" or (kicked and "short" or "no_kick"))
	end
end
