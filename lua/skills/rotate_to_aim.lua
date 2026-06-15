-- Rotate in place while holding the ball to aim at a target
-- Allows the robot to spin without moving significantly from its position

local rotate_to_aim = {}

local utils = require("utils.utils")
local has_the_ball = utils.has_the_ball

--- Calculates the shortest rotation angle to target
local function calculate_rotation(current_angle, target_angle)
	local diff = target_angle - current_angle
	
	-- Normalize to [-PI, PI]
	while diff > math.pi do
		diff = diff - (2 * math.pi)
	end
	while diff < -math.pi do
		diff = diff + (2 * math.pi)
	end
	
	return diff
end

--- Check if facing target within tolerance
local function is_facing_target(robot_state, target_pos, tolerance)
	local dx = target_pos.x - robot_state.x
	local dy = target_pos.y - robot_state.y
	local target_angle = math.atan(dy, dx)
	
	local angle_diff = math.abs(robot_state.orientation - target_angle)
	
	while angle_diff > math.pi do
		angle_diff = angle_diff - (2 * math.pi)
	end
	angle_diff = math.abs(angle_diff)
	
	return angle_diff <= tolerance
end

--- Main process: Rotate in place while maintaining ball control
-- @param robotId number - Robot ID
-- @param team number - Team number
-- @param target table - Target position to aim at {x, y}
-- @param tolerance number - Angle tolerance in radians (default: 0.1)
-- @return boolean - true when aimed, false otherwise
function rotate_to_aim.process(robotId, team, target, tolerance)
	tolerance = tolerance or 0.1
	
	local robot_state = get_robot_state(robotId, team)
	local ball_state = get_ball_state()
	
	-- If we don't have the ball, return false
	if not has_the_ball(robotId, team) then
		return false
	end
	
	-- Activate dribbler to maintain ball control
	dribbler(robotId, team, 5)
	
	-- Check if already facing target
	if is_facing_target(robot_state, target, tolerance) then
		return true
	end
	
	-- Rotate toward target
	-- Calculate rotation needed
	local dx = target.x - robot_state.x
	local dy = target.y - robot_state.y
	local target_angle = math.atan(dy, dx)
	
	local rotation_diff = calculate_rotation(robot_state.orientation, target_angle)
	
	-- Use face_to to rotate (with zero velocity to stay in place)
	face_to(robotId, team, target)
	
	-- Keep very low velocity to avoid drifting far from position
	-- send_velocity(robotId, team, 0, 0, 0)
	
	return false
end

return rotate_to_aim
