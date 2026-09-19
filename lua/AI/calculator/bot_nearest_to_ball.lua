local M = {}

--- Returns the single bot nearest the ball across both teams.
--- Each argument is a list of { robot, distance } ordered closest-first
--- (as produced by bot_distance_to_ball). On a tie the ally bot is preferred.
--- @param enemy_bots { robot: RobotState, distance: number }[]
--- @param ally_bots  { robot: RobotState, distance: number }[]
--- @return RobotState|nil  -- nearest robot, or nil if both lists are empty
function M.calc(enemy_bots, ally_bots)
	local ally = ally_bots[1]
	local enemy = enemy_bots[1]

	if not ally and not enemy then
		return nil
	end
	if not ally then
		return enemy.robot
	end
	if not enemy then
		return ally.robot
	end

	-- Tie (equal distance) prefers the ally bot: use <= on the ally distance.
	if ally.distance <= enemy.distance then
		return ally.robot
	end
	return enemy.robot
end

return M
