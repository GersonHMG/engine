-- Attacker role: the ball owner. Exactly one robot holds it (see ball_owner).
local Role = require("AI.roles.role")
local KickToGoal = require("AI.actions.kick_to_goal")
local KickToClear = require("AI.actions.kick_to_clear")
local Pass = require("AI.actions.pass")
local GoToBall = require("AI.actions.go_to_ball")

local SHOOT_THRESHOLD = 0.5  -- always shoot above this probability of scoring, if we can kick

local Attacker = {}

--- Forces kick_to_goal when the shot is good enough, but only with the ball
--- at our feet: without `can_kick` the robot would go for a shot it cannot take.
local function shoot_override(state, scored)
	if not state.can_kick or state.probability_of_score <= SHOOT_THRESHOLD then
		return nil
	end
	for i, entry in ipairs(scored) do
		if entry.action.name == "kick_to_goal" then
			return i
		end
	end
	return nil
end

--- @param team integer
--- @return Role
function Attacker.new(team)
	return Role.new("attacker", {
		KickToGoal.new(team),
		KickToClear.new(team),
		Pass.new(team),
		GoToBall.new(team),
	}, shoot_override)
end

return Attacker
