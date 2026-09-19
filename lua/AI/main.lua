-- Run framework experiment for one robot.
--
-- One tick = evaluate every candidate action for a single robot, score each
-- one, and execute the highest scoring one.

local world = require("AI.calculator.world")
local utils = require("utils.utils")


local M = {}

local TEAM = 0                            -- our team id (0 = blue, 1 = yellow)
local ROBOT_ID = 0                        -- the robot this experiment drives
local GOAL_TARGET = { x = 4.5, y = 0.0 }  -- enemy goal center
local KICK_RANGE = 0.25                   -- close enough to the ball to attempt a kick
local LABEL_OFFSET = 0.2                  -- height of the first score label above the robot
local LABEL_SPACING = 0.15                -- vertical gap between score labels

local KickToGoal = require("AI.actions.kick_to_goal")
local GoToBall = require("AI.actions.go_to_ball")
local DribbleToClear = require("AI.actions.dribble_to_clear")

local function to_action_entry(instance)
	return {
		name = instance.name,
		evaluate = function(state) return instance:evaluate(state) end,
		run = function(state) return instance:run(state) end,
	}
end

local ACTIONS = {
	to_action_entry(KickToGoal.new(TEAM)),
	to_action_entry(GoToBall.new(TEAM)),
	to_action_entry(DribbleToClear.new(TEAM))
}


--- Builds the state the action evaluations read from.
--- @param robot RobotState
--- @return table state
local function build_state(robot)
	local nearest = world.nearest_to_ball()

	return {
		robot = robot,
		ball = world.ball(),
		nearest_to_ball = nearest,
		-- Ball controlled in the dribbler, facing it: strict possession.
		has_ball = utils.has_the_ball(robot.id, TEAM),
		distance_to_ball = world.distance_to_ball(robot),
		-- Near enough to go for a kick: the kick skill drives the approach and
		-- the facing itself, so proximity alone is enough to consider it.
		can_kick = world.distance_to_ball(robot) <= KICK_RANGE,
		-- Closer to the goal is a better shot, and only if the shot line is
		-- not covered by an enemy.
		probability_of_score = world.probability_of_score(robot, GOAL_TARGET),
		-- We are more likely to win the ball when we are already the closest.
		probability_of_intercept = (nearest ~= nil and nearest.id == robot.id and nearest.team == TEAM) and 1.0 or 0.5,
	}
end

--- Action chosen on the previous tick, so we can stay committed to it.
local current_index = nil

--- Scores every action for the given state.
---
--- The winner has to beat the action we are already running by SWITCH_MARGIN.
--- Without that the robot flip-flops between two actions on nearly equal
--- scores and never finishes either approach, so it oscillates in place.
--- @param state table
--- @return { action: table, score: number }[] scored  -- same order as ACTIONS
--- @return integer best_index
local function score_actions(state)
	local scored = {}
	local best_index = 1

	for i, action in ipairs(ACTIONS) do
		scored[i] = { action = action, score = action.evaluate(state) }
		if scored[i].score > scored[best_index].score then
			best_index = i
		end
	end

	if current_index and best_index ~= current_index then
		-- Keep running the current action unless the challenger is clearly better.
		if scored[best_index].score < scored[current_index].score then
			best_index = current_index
		end
	end

	current_index = best_index
	return scored, best_index
end

--- Draws the state and the score of every action above the robot,
--- best action highlighted.
--- @param robot RobotState
--- @param state table
--- @param scored { action: table, score: number }[]
--- @param best_index integer
local function draw_scores(robot, state, scored, best_index)
	-- Possession flags, on top of the action labels.
	draw_text(
		robot.x,
		robot.y + LABEL_OFFSET + #scored * LABEL_SPACING,
		string.format(
			"has_ball: %s | can_kick: %s",
			tostring(state.has_ball),
			tostring(state.can_kick)
		),
		state.can_kick and { r = 0.0, g = 1.0, b = 0.0 } or { r = 1.0, g = 0.0, b = 0.0 }
	)

	for i, entry in ipairs(scored) do
		local color = (i == best_index) and { r = 0.0, g = 1.0, b = 0.0 } or { r = 0.6, g = 0.6, b = 0.6 }
		draw_text(
			robot.x,
			robot.y + LABEL_OFFSET + (#scored - i) * LABEL_SPACING,
			string.format("%s: %.3f", entry.action.name, entry.score),
			color
		)
	end
end

--- Entry point: run one tick of the experiment.
--- Called from the root entry script (see lua/run_ai.lua).
function M.process()
	world.update(TEAM) -- once per tick: one read of the engine

	local robot = world.robot(ROBOT_ID)
	if not robot then
		return
	end

	local state = build_state(robot)

	-- Best action for this robot, objective: score a goal.
	local scored, best_index = score_actions(state)
	draw_scores(robot, state, scored, best_index)

	scored[best_index].action.run(state)
end

return M