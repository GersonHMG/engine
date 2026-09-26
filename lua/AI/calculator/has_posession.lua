local M = {}

local POSSESSION_DISTANCE = 0.3 -- meters

---Returns true when the given robot is within possession range of the ball.
---@param robot RobotState
---@return boolean
function M.calc(robot)
	local ball = get_ball_state()
	local dx = robot.x - ball.x
	local dy = robot.y - ball.y
	local distance = math.sqrt(dx * dx + dy * dy)

	return distance <= POSSESSION_DISTANCE
end

return M
