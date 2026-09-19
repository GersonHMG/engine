local M = {}

---Returns your robots ordered by distance to the ball (closest first).
---@param team integer               -- your team id (0 = blue, 1 = yellow)
---@return { robot: RobotState, distance: number }[] ordered
function M.ordered_by_distance_to_ball(team)
	local ball = get_ball_state()
	local robots = (team == 0) and get_blue_team_state() or get_yellow_team_state()

	local ordered = {}
	for _, r in ipairs(robots) do
		if r.active then
			local dx = r.x - ball.x
			local dy = r.y - ball.y
			ordered[#ordered + 1] = {
				robot = r,
				distance = math.sqrt(dx * dx + dy * dy),
			}
		end
	end

	table.sort(ordered, function(a, b)
		return a.distance < b.distance
	end)

	return ordered
end

return M
