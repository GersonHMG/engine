-- DribbleToClear.lua
local Action = require("AI.actions.action")
local world = require("AI.calculator.world")
local skill_move_ball = require("skills.move_ball")

local DribbleToClear = setmetatable({}, { __index = Action })
DribbleToClear.__index = DribbleToClear

local OWN_GOAL = { x = -4.5, y = 0.0 }  -- our own goal center
local CLEAR_X = 0.0                     -- dribble the ball up to the halfway line
local WING_Y = 2.5                      -- how far out to the side the clear target sits
local LAMBDA = 0.35                     -- decay of urgency with distance to our own goal

--- Picks the wing (top or bottom) with fewer nearby opponents, so the clear
--- target is less likely to be walked into by an enemy.
--- @param opponents RobotState[]|nil
--- @return number  -- WING_Y or -WING_Y
local function open_wing_y(opponents)
	local top_count, bottom_count = 0, 0
	for _, enemy in ipairs(opponents or {}) do
		if enemy.y >= 0 then
			top_count = top_count + 1
		else
			bottom_count = bottom_count + 1
		end
	end
	return (top_count <= bottom_count) and WING_Y or -WING_Y
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
	local target = { x = CLEAR_X, y = open_wing_y(state.opponents) }
	skill_move_ball.process(state.robot.id, state.robot.team, target)
end

return DribbleToClear
