-- ReceivePass.lua
local Action = require("AI.actions.action")
local profile = require("utils.robot_profile")
local world = require("AI.calculator.world")
local tempo = require("AI.calculator.tempo")

local ReceivePass = setmetatable({}, { __index = Action })
ReceivePass.__index = ReceivePass

local DRIBBLER_SPEED = 5
local GOAL_TARGET = { x = 4.5, y = 0.0 }  -- enemy goal center
-- Fraction of the kicker face's acceptance angle (robot profile) the receiver may turn
-- from the ball towards the goal: the ball must still hit the face.
local TURN_MARGIN = 0.8

local function wrap(angle)
	while angle > math.pi do angle = angle - 2 * math.pi end
	while angle <= -math.pi do angle = angle + 2 * math.pi end
	return angle
end

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

--- One-touch shot: the ball can be kicked on the first touch when it comes in within the
--- profile's one_touch_angle of the shot, and the goal is within a kick's roll. The robot
--- then puts its kicker face (not its center) on the ball's path, faces the aim with the
--- largest margin and arms the kicker: the kick fires when the ball reaches the face, and
--- leaves along the heading whatever the ball's incoming direction.
--- @return boolean  -- true when it is going for a one-touch shot this tick
local function one_touch(robot, ball, path_point)
	if not path_point then return false end
	if world.distance(path_point, tempo.GOAL) > profile.roll_distance() then return false end
	local aim = tempo.best_aim(path_point, 0.0, world.active_enemies())
	if not tempo.one_touch(path_point, ball, aim) then return false end

	local heading = math.atan(aim.y - path_point.y, aim.x - path_point.x)
	local c = profile.contact_distance()
	local spot = { x = path_point.x - c * math.cos(heading), y = path_point.y - c * math.sin(heading) }
	draw_point(spot.x, spot.y, true, { r = 1.0, g = 0.0, b = 1.0 })
	draw_line({ path_point, aim }, { r = 1.0, g = 0.0, b = 1.0 })
	move_direct(robot.id, robot.team, spot)
	face_to(robot.id, robot.team, aim)
	kickx(robot.id, robot.team)
	return true
end

--- Steps onto the ball's path (the closest point of its trajectory line). It shoots on the
--- first touch when it can (one_touch); otherwise it faces the ball turned towards the goal
--- as far as the kicker face allows (less to turn before the shot) and spins the dribbler
--- to trap it.
function ReceivePass:run(state)
	local robot, ball = state.robot, state.ball
	local target = { x = robot.x, y = robot.y }
	local path_point = nil

	local speed = math.sqrt(ball.vel_x ^ 2 + ball.vel_y ^ 2)
	if speed > 0.1 then
		local dir_x, dir_y = ball.vel_x / speed, ball.vel_y / speed
		local along = (robot.x - ball.x) * dir_x + (robot.y - ball.y) * dir_y
		if along > 0 then
			target = { x = ball.x + dir_x * along, y = ball.y + dir_y * along }
			path_point = target
		end
	end

	if one_touch(robot, ball, path_point) then
		return
	end

	draw_point(target.x, target.y, true, { r = 0.0, g = 1.0, b = 0.0 })
	move_to(robot.id, robot.team, target)

	local to_ball = math.atan(ball.y - robot.y, ball.x - robot.x)
	local to_goal = math.atan(GOAL_TARGET.y - robot.y, GOAL_TARGET.x - robot.x)
	local max_turn = TURN_MARGIN * profile.kicker_acceptance()
	local turn = math.max(-max_turn, math.min(max_turn, wrap(to_goal - to_ball)))
	local heading = to_ball + turn
	face_to(robot.id, robot.team, { x = robot.x + math.cos(heading), y = robot.y + math.sin(heading) })
	dribbler(robot.id, robot.team, DRIBBLER_SPEED)
end

return ReceivePass
