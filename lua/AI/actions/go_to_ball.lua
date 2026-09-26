-- GoToBall.lua
local Action = require("AI.actions.action")
local skill = require("skills.go_to_ball")
local GoToBall = setmetatable({}, { __index = Action })
GoToBall.__index = GoToBall

local LAMBDA = 0.35  -- decay of the probability with distance to ball

--- @param team integer
--- @return GoToBall
function GoToBall.new(team)
	local self = setmetatable(Action.new("go_to_ball"), GoToBall)
	self.team = team
	return self
end


-- P(Score | s , go) = P( B | s, a ) * V(hold_the_ball)
-- = P( success ) * V(hold_the_ball)
function GoToBall:evaluate(state)
	local decay = math.exp(-LAMBDA * state.distance_to_ball)
	local k = state.can_kick and 1.0 or 0.0
	
	return (1 - k)
end

function GoToBall:run(state)
	-- TODO: point this at whatever movement primitive you use now.
	-- e.g. world.set_move_command(state.robot.id, self.team, state.ball)
    skill.process(state.robot.id, state.robot.team)
end

return GoToBall