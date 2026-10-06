-- Attacker role: the ball owner. Exactly one robot holds it (see ball_owner).
local Role = require("AI.roles.role")
local profile = require("utils.robot_profile")
local KickToGoal = require("AI.actions.kick_to_goal")
local KickToClear = require("AI.actions.kick_to_clear")
local Pass = require("AI.actions.pass")
local GoToBall = require("AI.actions.go_to_ball")

local SHOOT_THRESHOLD = 0.5  -- always shoot above this probability of scoring, if we can kick
local GOAL = { x = 4.5, y = 0.0 }  -- enemy goal center
local SHOOT_IN_RANGE = 0.15  -- within kick range, shoot above this probability of scoring

local Attacker = {}

--- Forces kick_to_goal when the shot is good enough, but only with the ball
--- at our feet: without `can_kick` the robot would go for a shot it cannot take.
--- Within kick range a lower probability is enough: a shot ready now beats a
--- pass that needs time to line up, receive and turn while the defender closes.
local function shoot_override(state, scored)
	if not state.can_kick then
		return nil
	end
	local r = state.robot
	-- In range: the goal is closer than a kick rolls (robot profile).
	local in_range = math.sqrt((GOAL.x - r.x) ^ 2 + (GOAL.y - r.y) ^ 2) <= profile.roll_distance()
	local threshold = in_range and SHOOT_IN_RANGE or SHOOT_THRESHOLD
	if state.probability_of_score <= threshold then
		return nil
	end
	for i, entry in ipairs(scored) do
		if entry.action.name == "kick_to_goal" then
			return i
		end
	end
	return nil
end

--- @param team integer
--- @return Role
function Attacker.new(team)
	return Role.new("attacker", {
		KickToGoal.new(team),
		KickToClear.new(team),
		Pass.new(team),
		GoToBall.new(team),
	}, shoot_override)
end

return Attacker
