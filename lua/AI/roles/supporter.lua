-- Supporter role: every ally that does not own the ball.
local Role = require("AI.roles.role")
local ReceivePass = require("AI.actions.receive_pass")
local PositionForPass = require("AI.actions.position_for_pass")

local Supporter = {}

--- @param team integer
--- @return Role
function Supporter.new(team)
	return Role.new("supporter", {
		ReceivePass.new(team),
		PositionForPass.new(team),
	})
end

return Supporter
