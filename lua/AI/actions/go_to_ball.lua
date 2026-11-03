-- GoToBall.lua
local Action = require("AI.actions.action")
local skill = require("skills.go_to_ball")
local GoToBall = setmetatable({}, { __index = Action })
GoToBall.__index = GoToBall

local GOAL_TARGET = { x = 4.5, y = 0.0 }  -- enemy goal center: arrive at the ball lined up to shoot

--- @param team integer
--- @return GoToBall
function GoToBall.new(team)
	local self = setmetatable(Action.new("go_to_ball"), GoToBall)
	self.team = team
	return self
end


local UNAVAILABLE = -1.0  -- with the ball in kick range: waiting next to it is not an option

--- The attacker without the ball has nothing else to do: 1. With the ball in kick range
--- the kicks take over, even when every kick has a negative expected value (waiting does
--- not make them better, and the time runs out).
function GoToBall:evaluate(state)
	return state.can_kick and UNAVAILABLE or 1.0
end

--- Done once the ball is in kick range: the kick options take over from there.
function GoToBall:terminated(state)
	return state.can_kick
end

function GoToBall:run(state)
	-- TODO: point this at whatever movement primitive you use now.
	-- e.g. world.set_move_command(state.robot.id, self.team, state.ball)
    skill.process(state.robot.id, state.robot.team, GOAL_TARGET)
end

return GoToBall