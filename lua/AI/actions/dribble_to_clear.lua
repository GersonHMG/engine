-- DribbleToClear.lua
local Action = require("AI.actions.action")
local world = require("AI.calculator.world")
local skill_move_ball = require("skills.move_ball")

local DribbleToClear = setmetatable({}, { __index = Action })
DribbleToClear.__index = DribbleToClear

local OWN_GOAL = { x = -4.5, y = 0.0 }  -- our own goal center
local CLEAR_DISTANCE = 0.8               -- how far to dribble away from our goal, per clear
local LAMBDA = 0.35                      -- decay of urgency with distance to our own goal
local BLOCK_RADIUS = 0.20                 -- clearance an enemy needs from the travel path
local MAX_SIDESTEP = 1.0                 -- cap on how far the target gets deflected sideways

--- Target relative to the robot's current position: primarily straight out
--- along the line from our own goal through the robot, then deflected
--- sideways around any enemy that actually sits on that path, so the ball
--- goes around them instead of through them. Scaled to a fixed, short
--- distance so every clear costs roughly the same travel.
--- @param robot RobotState
--- @param opponents RobotState[]|nil
--- @return { x: number, y: number }
local function clear_target(robot, opponents)
	local dx = robot.x - OWN_GOAL.x
	local dy = robot.y - OWN_GOAL.y
	local length = math.sqrt(dx * dx + dy * dy)

	if length < 0.001 then
		-- Directly on our own goal: fall back to straight upfield.
		dx, dy, length = 1.0, 0.0, 1.0
	end

	local dir_x, dir_y = dx / length, dy / length
	-- Perpendicular to the escape direction, used to sidestep blockers.
	local perp_x, perp_y = -dir_y, dir_x

	-- For every enemy that actually lies on the segment from the robot to
	-- the (fixed-length) clear target, push the target sideways just far
	-- enough to keep BLOCK_RADIUS of clearance from it.
	local sidestep = 0.0
	for _, enemy in ipairs(opponents or {}) do
		local ex, ey = enemy.x - robot.x, enemy.y - robot.y
		local along = dir_x * ex + dir_y * ey
		if along > 0.0 and along < CLEAR_DISTANCE then
			local offset = perp_x * ex + perp_y * ey
			if math.abs(offset) < BLOCK_RADIUS then
				local needed = BLOCK_RADIUS - math.abs(offset)
				local sign = (offset >= 0.0) and -1.0 or 1.0
				sidestep = sidestep + sign * needed
			end
		end
	end
	sidestep = math.max(-MAX_SIDESTEP, math.min(MAX_SIDESTEP, sidestep))

	return {
		x = robot.x + dir_x * CLEAR_DISTANCE + perp_x * sidestep,
		y = robot.y + dir_y * CLEAR_DISTANCE + perp_y * sidestep,
	}
end

--- @param team integer
--- @return DribbleToClear
function DribbleToClear.new(team)
	local self = setmetatable(Action.new("dribble_to_clear"), DribbleToClear)
	self.team = team
	return self
end

--- Only worth clearing while we actually have the ball, and more urgent the
--- closer we are to our own goal.
function DribbleToClear:evaluate(state)
	local h = state.has_ball and 1.0 or 0.0
	local distance_to_own_goal = world.distance(state.robot, OWN_GOAL)
	local urgency = math.exp(-LAMBDA * distance_to_own_goal)
	return h * urgency
end

function DribbleToClear:run(state)
	local opponents = state.opponents or world.active_enemies()
	local target = clear_target(state.robot, opponents)
	skill_move_ball.process(state.robot.id, state.robot.team, target)
end

return DribbleToClear
