local calc = require("AI.calculator.calculator")

local M = {}

-- Built desire bot map, it assigns each role to each robot

-- function
-- Assign keeper
-- Desired Defenders Calc

---Assigns a role to each of your robots. Proof-of-concept: everyone is "Offense".
---@param team integer                  -- your team id (0 = blue, 1 = yellow)
---@return table<integer, string> roles  -- map: botID -> role
---@return { id: integer, role: string, distance: number }[] ordered  -- ranked, closest-to-ball first
function M.assign_plays(team)
	local ranked = calc.bot_to_ball_distance(team, false)

	local roles = {}    -- id -> role  (fast lookup)
	local ordered = {}  -- keeps the closest-first ordering

	for _, entry in ipairs(ranked) do
		local id = entry.robot.id
		local role = "Offense"  -- everyone is Offense for this POC

		roles[id] = role
		ordered[#ordered + 1] = {
			id = id,
			role = role,
			distance = entry.distance,
		}
	end

	return roles, ordered
end

return M