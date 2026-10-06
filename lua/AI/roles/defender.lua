-- Defender role: every robot that is not goalkeeper, attacker or supporter. Marks an
-- enemy or guards the ball (see role_assigner for the marking assignment).
local Role = require("AI.roles.role")
local Defend = require("AI.actions.defend")

local Defender = {}

--- @param team integer
--- @return Role
function Defender.new(team)
	return Role.new("defender", { Defend.new(team) })
end

return Defender
