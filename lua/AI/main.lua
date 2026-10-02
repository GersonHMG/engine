-- Run framework experiment for a team.
--
-- One tick:
--   1. Snapshot the world.
--   2. Pick the ball owner (lowest time to ball, with hysteresis).
--   3. Owner plays Attacker, every other robot plays Supporter.
--   4. Each robot scores its role's actions and runs the best one.

local world = require("AI.calculator.world")
local utils = require("utils.utils")
local has_posession = require("AI.calculator.has_posession")
local ball_owner = require("AI.calculator.ball_owner")
local Attacker = require("AI.roles.attacker")
local Supporter = require("AI.roles.supporter")

local M = {}

local TEAM = 0                            -- our team id (0 = blue, 1 = yellow)
local ROBOT_IDS = { 0, 1 }                -- the robots this experiment drives
local GOAL_TARGET = { x = 4.5, y = 0.0 }  -- enemy goal center
local LABEL_OFFSET = 0.2                  -- height of the first score label above the robot
local LABEL_SPACING = 0.15                -- vertical gap between score labels

--- Per robot: its own role instances and which one ran last tick.
--- @type table<integer, { attacker: Role, supporter: Role, active: string|nil }>
local slots = {}

local function slot_for(id)
	if not slots[id] then
		slots[id] = {
			attacker = Attacker.new(TEAM),
			supporter = Supporter.new(TEAM),
			active = nil,
		}
	end
	return slots[id]
end

--- Our driven robots that are on the field this tick.
--- @return RobotState[]
local function team_robots()
	local robots = {}
	for _, id in ipairs(ROBOT_IDS) do
		local r = world.robot(id)
		if r and r.active then
			robots[#robots + 1] = r
		end
	end
	return robots
end

--- Builds the state the action evaluations read from.
--- @param robot RobotState
--- @param robots RobotState[]  -- the whole team, robot included
--- @return table state
local function build_state(robot, robots)
	local teammates = {}
	for _, r in ipairs(robots) do
		if r.id ~= robot.id then
			teammates[#teammates + 1] = r
		end
	end

	return {
		robot = robot,
		ball = world.ball(),
		teammates = teammates,
		opponents = world.active_enemies(),
		-- Ball controlled in the dribbler, facing it: strict possession.
		has_ball = utils.has_the_ball(robot.id, TEAM),
		distance_to_ball = world.distance_to_ball(robot),
		-- Near enough to go for a kick: the kick skill drives the approach and
		-- the facing itself, so proximity alone is enough to consider it.
		can_kick = has_posession.calc(robot),
		probability_of_score = world.probability_of_score(robot, GOAL_TARGET),
		is_owner = ball_owner.is_owner(robot),
		is_receiver = ball_owner.is_receiver(robot),
	}
end

--- Draws the role and the score of every action above the robot,
--- best action highlighted.
--- @param robot RobotState
--- @param role Role
--- @param scored { action: table, score: number }[]
--- @param best_index integer
local function draw_scores(robot, role, scored, best_index)
	draw_text(
		robot.x,
		robot.y + LABEL_OFFSET + #scored * LABEL_SPACING,
		role.name,
		role.name == "attacker" and { r = 1.0, g = 0.6, b = 0.0 } or { r = 0.0, g = 0.6, b = 1.0 }
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

	local robots = team_robots()
	ball_owner.update(robots)

	for _, robot in ipairs(robots) do
		local slot = slot_for(robot.id)
		local role_name = ball_owner.is_owner(robot) and "attacker" or "supporter"
		if slot.active ~= role_name then
			slot[role_name]:reset()
			slot.active = role_name
		end

		local role = slot[role_name]
		local state = build_state(robot, robots)
		local scored, best_index = role:choose(state)
		draw_scores(robot, role, scored, best_index)

		scored[best_index].action:run(state)
	end
end

return M
