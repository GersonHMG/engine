-- KeepGoal.lua
-- The goalkeeper's only action: stand on our goal line between the ball and the goal
-- (skills/goalkeeper).
local Action = require("AI.actions.action")
local skill = require("skills.goalkeeper")

local KeepGoal = setmetatable({}, { __index = Action })
KeepGoal.__index = KeepGoal

local OWN_GOAL = { x = -4.5, y = 0.0 }  -- our goal center

--- @param team integer
--- @return KeepGoal
function KeepGoal.new(team)
	local self = setmetatable(Action.new("keep_goal"), KeepGoal)
	self.team = team
	return self
end

function KeepGoal:evaluate(state)
	return 1.0
end

function KeepGoal:run(state)
	skill.process(state.robot.id, state.robot.team, OWN_GOAL)
end

return KeepGoal
