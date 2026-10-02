--- ball_owner.lua -- decides which ally owns the ball, so only one goes for it.
---
--- Owner = the robot with the lowest time to reach the ball:
---
---     T_i = d(r_i, b̂) / V_MAX + |wrap(θ_i − atan2(b̂ − r_i))| / OMEGA_MAX
---     b̂   = b + v_b · LOOKAHEAD                 (where the ball will be)
---
--- with hysteresis: a challenger only takes over when
---
---     T_challenger < T_owner − SWITCH_MARGIN
---
--- so two robots at nearly equal times never trade ownership back and forth.
---
--- While a pass is in flight (ball moving fast towards an ally) nobody owns
--- the ball: the passer must not chase its own pass, and the receiver handles
--- it as a supporter. The receiver is remembered as owner, so it keeps the
--- ball when the pass arrives.
---
--- Call `update(robots)` once per tick, after `world.update`.

local world = require("AI.calculator.world")

local M = {}

-- Tuning knobs.
M.V_MAX = 2.0            -- m/s, robot cruise speed used for the time estimate
M.OMEGA_MAX = 4.0        -- rad/s, robot turn rate used for the time estimate
M.LOOKAHEAD = 0.3        -- s, how far ahead the ball position is predicted
M.SWITCH_MARGIN = 0.25   -- s, how much faster a challenger has to be
M.PASS_MIN_SPEED = 1.0   -- m/s, slower than this the ball is not "in flight" (dribbling pushes are slower)
M.PASS_LANE_WIDTH = 0.5  -- m, max distance of an ally from the ball's path

M.owner = nil     -- RobotState|nil, the ally that owns the ball this tick
M.receiver = nil  -- RobotState|nil, the ally a pass in flight is heading to

local owner_id = nil  -- owner remembered across ticks, for hysteresis

local function wrap(angle)
	while angle > math.pi do angle = angle - 2 * math.pi end
	while angle <= -math.pi do angle = angle + 2 * math.pi end
	return angle
end

--- Estimated seconds for `robot` to reach `target`: drive + turn to face it.
--- @param robot RobotState
--- @param target { x: number, y: number }
--- @return number
function M.time_to_ball(robot, target)
	local heading = math.atan(target.y - robot.y, target.x - robot.x)
	local turn = math.abs(wrap(robot.orientation - heading))
	return world.distance(robot, target) / M.V_MAX + turn / M.OMEGA_MAX
end

--- @return { x: number, y: number }
local function predicted_ball()
	local b = world.ball()
	return { x = b.x + b.vel_x * M.LOOKAHEAD, y = b.y + b.vel_y * M.LOOKAHEAD }
end

--- The ally the moving ball is heading to, if any: ahead of the ball along
--- its velocity and within PASS_LANE_WIDTH of its path. The passer is behind
--- the ball once it leaves, so it is never picked.
--- @param robots RobotState[]
--- @return RobotState|nil
function M.incoming_receiver(robots)
	local b = world.ball()
	local speed = world.ball_speed()
	if speed < M.PASS_MIN_SPEED then
		return nil
	end

	local dir_x, dir_y = b.vel_x / speed, b.vel_y / speed
	local best, best_along = nil, math.huge

	for _, r in ipairs(robots) do
		local rx, ry = r.x - b.x, r.y - b.y
		local along = dir_x * rx + dir_y * ry
		local offset = math.abs(dir_x * ry - dir_y * rx)
		if along > 0.0 and offset < M.PASS_LANE_WIDTH and along < best_along then
			best, best_along = r, along
		end
	end

	return best
end

--- Recomputes `M.owner` and `M.receiver` for this tick.
--- @param robots RobotState[]  -- the allies taking part, active only
function M.update(robots)
	M.receiver = M.incoming_receiver(robots)
	if M.receiver then
		owner_id = M.receiver.id
		M.owner = nil
		return
	end

	local target = predicted_ball()

	-- argmin T_i; on equal times the first robot in the list wins.
	local best, best_t = nil, math.huge
	for _, r in ipairs(robots) do
		local t = M.time_to_ball(r, target)
		if t < best_t then
			best, best_t = r, t
		end
	end

	-- Hysteresis: keep the current owner unless the challenger is clearly faster.
	local current = nil
	for _, r in ipairs(robots) do
		if r.id == owner_id then current = r end
	end
	if current and best and best.id ~= current.id then
		if best_t >= M.time_to_ball(current, target) - M.SWITCH_MARGIN then
			best = current
		end
	end

	owner_id = best and best.id or nil
	M.owner = best
end

--- @param robot RobotState
--- @return boolean
function M.is_owner(robot)
	return M.owner ~= nil and M.owner.id == robot.id
end

--- @param robot RobotState
--- @return boolean
function M.is_receiver(robot)
	return M.receiver ~= nil and M.receiver.id == robot.id
end

return M
