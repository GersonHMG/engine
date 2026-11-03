-- PositionForPass.lua
--
-- The supporter's move: stand where a pass to it is worth the most (docs/action-scoring.md,
-- section 3.7). Every candidate spot p of a grid is scored as the pass the ball owner could
-- play to it once the supporter is there (AI/calculator/value.pass_to with t_ready = the
-- supporter's time to reach p): lane, control, and what the ball is worth at p (its best
-- shot or V). Spots inside or near a defense area, or too close to the ball, are skipped.
-- The target is re-planned every REPLAN_TICKS and only replaced when a new spot beats it by
-- KEEP_MARGIN.
local Action = require("AI.actions.action")
local world = require("AI.calculator.world")
local value = require("AI.calculator.value")
local ball_owner = require("AI.calculator.ball_owner")
local area_guard = require("utils.area_guard")
local profile = require("utils.robot_profile")

local PositionForPass = setmetatable({}, { __index = Action })
PositionForPass.__index = PositionForPass

-- Candidate grid: the attacking side of the field.
local GRID_STEP = 0.5      -- m
local X_MIN, X_MAX = -1.0, 3.4
local Y_MIN, Y_MAX = -2.6, 2.6
local MIN_BALL_DIST = 1.0  -- m, closer than this to the ball is the attacker's space
local KEEP_MARGIN = 0.02   -- a new spot must beat the current one by this much
local REPLAN_TICKS = 15    -- ticks between two re-plans

local BASE_SCORE = 0.5     -- always available, below receive_pass (1.0)

--- @param team integer
--- @return PositionForPass
function PositionForPass.new(team)
	local self = setmetatable(Action.new("position_for_pass"), PositionForPass)
	self.team = team
	self.target = nil
	self.ticks = 0
	return self
end

function PositionForPass:evaluate(state)
	return BASE_SCORE
end

--- The robot the ball would be passed from: the owner, else our robot nearest the ball.
local function passer(state)
	if ball_owner.owner and ball_owner.owner.id ~= state.robot.id then
		return ball_owner.owner
	end
	local best, best_d = nil, math.huge
	for _, r in ipairs(state.teammates or {}) do
		local d = world.distance(r, state.ball)
		if d < best_d then best, best_d = r, d end
	end
	return best
end

--- Value of the supporter standing at `p`: the pass to it, or nil when not possible.
local function spot_value(p, robot, from, ball, enemies)
	if world.distance(p, ball) < MIN_BALL_DIST then return nil end
	if area_guard.touched(p, area_guard.MARGIN) then return nil end
	local mate = { id = robot.id, x = p.x, y = p.y, orientation = robot.orientation }
	local t_ready = profile.time_to_reach(robot, p)
	-- The passer as if already facing p: the spot must not depend on where the passer
	-- happens to point now (it turns between options, and the spot would follow it).
	local facing = { x = from.x, y = from.y, vel_x = from.vel_x, vel_y = from.vel_y,
		orientation = math.atan(p.y - ball.y, p.x - ball.x) }
	local option = value.pass_to(facing, ball, mate, enemies, t_ready)
	return option and option.score or nil
end

function PositionForPass:run(state)
	local robot, ball = state.robot, state.ball
	local enemies = state.opponents or world.active_enemies()
	local from = passer(state)

	self.ticks = self.ticks - 1
	if from and (self.ticks <= 0 or not self.target) then
		self.ticks = REPLAN_TICKS
		local best, best_value = nil, -math.huge
		for x = X_MIN, X_MAX + 1e-9, GRID_STEP do
			for y = Y_MIN, Y_MAX + 1e-9, GRID_STEP do
				local p = { x = x, y = y }
				local v = spot_value(p, robot, from, ball, enemies)
				if v and v > best_value then
					best, best_value = p, v
				end
			end
		end
		if self.target then
			local current = spot_value(self.target, robot, from, ball, enemies)
			if current and best_value < current + KEEP_MARGIN then
				best = self.target
			end
		end
		self.target = best or self.target
	end

	if not self.target then return end
	draw_point(self.target.x, self.target.y, true, { r = 0.0, g = 0.6, b = 1.0 })
	move_to(robot.id, robot.team, self.target)
	face_to(robot.id, robot.team, { x = ball.x, y = ball.y })
end

return PositionForPass
