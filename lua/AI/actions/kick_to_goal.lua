-- KickToGoal.lua
local Action = require("AI.actions.action")
local world = require("AI.calculator.world")
local skill_kick = require("skills.kick_to_point")

local KickToGoal = setmetatable({}, { __index = Action })
KickToGoal.__index = KickToGoal

local GOAL_TARGET = { x = 4.5, y = 0.0 }  -- enemy goal center
local GOAL_HALF_WIDTH = 1.0               -- half the goal mouth width (post to post = 2x this)
local LAMBDA = 0.35                       -- decay of the probability with distance to goal

local GOAL_POSTS = {
	{ x = GOAL_TARGET.x, y = GOAL_TARGET.y - GOAL_HALF_WIDTH },
	{ x = GOAL_TARGET.x, y = GOAL_TARGET.y + GOAL_HALF_WIDTH },
}

--- Angle, in radians, subtended by segment (a, b) as seen from point p.
--- @param p { x: number, y: number }
--- @param a { x: number, y: number }
--- @param b { x: number, y: number }
--- @return number angle in [0, pi]
local function angle_subtended(p, a, b)
	local function angle_to(point)
		return math.atan(point.y - p.y, point.x - p.x)
	end
	local diff = angle_to(b) - angle_to(a)
	while diff > math.pi do
		diff = diff - 2 * math.pi
	end
	while diff <= -math.pi do
		diff = diff + 2 * math.pi
	end
	return math.abs(diff)
end

--- Fraction of the goal mouth's angular width, as seen from the robot, that
--- is NOT blocked by any opponent standing between the robot and the goal.
--- @param robot RobotState
--- @param opponents RobotState[]|nil
--- @return number fraction in [0, 1]
local function open_angle_fraction(robot, opponents)
	local total_angle = angle_subtended(robot, GOAL_POSTS[1], GOAL_POSTS[2])
	if total_angle <= 0 then
		return 0.0
	end

	local blocked_angle = 0.0
	for _, enemy in ipairs(opponents or {}) do
		if enemy.x > robot.x and enemy.x < GOAL_TARGET.x then
			local enemy_angle = angle_subtended(robot, enemy, GOAL_TARGET)
			blocked_angle = blocked_angle + math.min(enemy_angle, total_angle)
		end
	end

	local open = (total_angle - blocked_angle) / total_angle
	return math.max(0.0, math.min(1.0, open))
end

--- Probability of scoring a goal by kicking right now, from the robot's
--- current position: how open the goal mouth is, discounted by distance,
--- and zeroed out if a robot sits directly in front on the shot line.
--- @param robot RobotState
--- @param opponents RobotState[]|nil
--- @return number probability in [0, 1]
local function probability_of_score(robot, opponents)
	local distance = world.distance(robot, GOAL_TARGET)
	local distance_factor = math.exp(-LAMBDA * distance)
	local angle_factor = open_angle_fraction(robot, opponents)
	-- Catches a robot immediately in front of the shooter, blocking the
	-- straight line to the goal center, which the post-to-post angle check
	-- above can miss when the blocker is very close to the shooter.
	local line_factor = world.shot_clearance(robot, GOAL_TARGET)
	return distance_factor * angle_factor * line_factor
end

--- @param team integer
--- @return KickToGoal
function KickToGoal.new(team)
	local self = setmetatable(Action.new("kick_to_goal"), KickToGoal)
	self.team = team
	return self
end

function KickToGoal:evaluate(state)
	local k = state.can_kick and 1.0 or 0.0
	-- state.opponents isn't populated by every caller yet; fall back to the
	-- world snapshot's active enemies so the front-blocker check still works.
	local opponents = state.opponents or world.active_enemies()
	return k * probability_of_score(state.robot, opponents)
end

function KickToGoal:run(state)
	-- TODO: replace with your kick primitive now that kick_to_point is gone.
	-- e.g. world.set_kick_command(state.robot.id, self.team, GOAL_TARGET)
	skill_kick.process(state.robot.id, state.robot.team, GOAL_TARGET)
end

return KickToGoal