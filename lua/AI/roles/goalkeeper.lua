-- Goalkeeper role: guards our goal (see role_assigner for who plays it).
local Role = require("AI.roles.role")
local KeepGoal = require("AI.actions.keep_goal")

local Goalkeeper = {}

--- @param team integer
--- @return Role
function Goalkeeper.new(team)
	return Role.new("goalkeeper", { KeepGoal.new(team) })
end

return Goalkeeper
