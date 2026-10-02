-- PositionForPass.lua
local Action = require("AI.actions.action")
local world = require("AI.calculator.world")
local lane = require("AI.calculator.lane")

local PositionForPass = setmetatable({}, { __index = Action })
PositionForPass.__index = PositionForPass

local GOAL_TARGET = { x = 4.5, y = 0.0 }  -- enemy goal center

-- Candidate grid: the attacking side of the field, outside the enemy defense area.
local GRID_STEP = 0.25
local X_MIN, X_MAX = -1.0, 3.5
local Y_MIN, Y_MAX = -2.5, 2.5
local DEFENSE_X, DEFENSE_HALF_Y = 3.4, 1.1

-- F(p) weights.
local W_SCORE = 1.0      -- good spot to shoot from
local W_LANE = 0.5       -- open passing lane from the ball
local W_TRAVEL = 0.1     -- per meter the robot has to travel
local W_FAR = 0.2        -- per meter beyond MAX_PASS_DIST
local MIN_BALL_DIST = 1.0  -- closer than this to the ball is forbidden (attacker's space)
local MAX_PASS_DIST = 3.5  -- longer passes are penalized
local KEEP_MARGIN = 0.05   -- a new spot must beat the current one by this much

local BASE_SCORE = 0.5   -- always available, below receive_pass (1.0)

--- Support value of standing at `p`:
---     F(p) = W_SCORE·P_s(p) + W_LANE·Λ(b, p, E) − W_TRAVEL·d(r, p) − W_FAR·max(0, d(b, p) − MAX_PASS_DIST)
--- or −∞ inside the forbidden zones.
local function support_value(p, robot, ball, opponents)
	local d_ball = world.distance(p, ball)
	if d_ball < MIN_BALL_DIST then
		return -math.huge
	end
	if p.x > DEFENSE_X and math.abs(p.y) < DEFENSE_HALF_Y then
		return -math.huge
	end

	return W_SCORE * world.probability_of_score(p, GOAL_TARGET)
		+ W_LANE * lane.clear(ball, p, opponents)
		- W_TRAVEL * world.distance(robot, p)
		- W_FAR * math.max(0.0, d_ball - MAX_PASS_DIST)
end

--- @param team integer
--- @return PositionForPass
function PositionForPass.new(team)
	local self = setmetatable(Action.new("position_for_pass"), PositionForPass)
	self.team = team
	self.target = nil
	return self
end

function PositionForPass:evaluate(state)
	return BASE_SCORE
end

--- Picks the grid point with the highest F(p), keeping the current target
--- unless a new one beats it by KEEP_MARGIN, then moves there facing the ball.
function PositionForPass:run(state)
	local robot, ball = state.robot, state.ball
	local opponents = state.opponents or world.active_enemies()

	local best, best_value = nil, -math.huge
	for x = X_MIN, X_MAX + 1e-9, GRID_STEP do
		for y = Y_MIN, Y_MAX + 1e-9, GRID_STEP do
			local p = { x = x, y = y }
			local v = support_value(p, robot, ball, opponents)
			if v > best_value then
				best, best_value = p, v
			end
		end
	end

	if self.target then
		local current_value = support_value(self.target, robot, ball, opponents)
		if best_value < current_value + KEEP_MARGIN then
			best = self.target
		end
	end
	self.target = best

	if not self.target then return end
	draw_point(self.target.x, self.target.y, true, { r = 0.0, g = 0.6, b = 1.0 })
	move_to(robot.id, robot.team, self.target)
	face_to(robot.id, robot.team, { x = ball.x, y = ball.y })
end

return PositionForPass
