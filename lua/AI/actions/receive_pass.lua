-- ReceivePass.lua
local Action = require("AI.actions.action")

local ReceivePass = setmetatable({}, { __index = Action })
ReceivePass.__index = ReceivePass

local DRIBBLER_SPEED = 5

--- @param team integer
--- @return ReceivePass
function ReceivePass.new(team)
	local self = setmetatable(Action.new("receive_pass"), ReceivePass)
	self.team = team
	return self
end

--- 1 while a pass is in flight towards this robot (see ball_owner), else 0.
function ReceivePass:evaluate(state)
	return state.is_receiver and 1.0 or 0.0
end

--- Steps onto the ball's path (the closest point of its trajectory line),
--- faces the ball and spins the dribbler to trap it.
function ReceivePass:run(state)
	local robot, ball = state.robot, state.ball
	local target = { x = robot.x, y = robot.y }

	local speed = math.sqrt(ball.vel_x ^ 2 + ball.vel_y ^ 2)
	if speed > 0.1 then
		local dir_x, dir_y = ball.vel_x / speed, ball.vel_y / speed
		local along = (robot.x - ball.x) * dir_x + (robot.y - ball.y) * dir_y
		if along > 0 then
			target = { x = ball.x + dir_x * along, y = ball.y + dir_y * along }
		end
	end

	draw_point(target.x, target.y, true, { r = 0.0, g = 1.0, b = 0.0 })
	move_to(robot.id, robot.team, target)
	face_to(robot.id, robot.team, { x = ball.x, y = ball.y })
	dribbler(robot.id, robot.team, DRIBBLER_SPEED)
end

return ReceivePass
