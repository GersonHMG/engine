-- KickAction.lua
--
-- Base for options that end in a kick (KickToGoal, KickToClear, Pass).
-- They share one termination condition β:
--   * done:   the ball left at kick speed, or
--   * failed: the ball has been out of kick range for LOST_TICKS in a row.
--
-- The grace period is what stops the go_to_ball <-> kick loop: lining up
-- the kick nudges the ball out of range for a tick or two, and that alone
-- must not end the option.

local Action = require("AI.actions.action")
local world = require("AI.calculator.world")
local profile = require("utils.robot_profile")
local skill_kick = require("skills.kick_to_point")
local skill_orbit = require("skills.orbit_ball")

local KickAction = setmetatable({}, { __index = Action })
KickAction.__index = KickAction

local KICKED_SPEED = 1.0  -- m/s, faster than this the ball was kicked
local LOST_TICKS = 30     -- ticks out of kick range before giving up (≈0.5 s at 60 Hz)
local PRESSURE_S = 3.0    -- s: an enemy that can reach the ball this soon puts the kicker under pressure

--- @param name string
--- @return KickAction
function KickAction.new(name)
	local self = setmetatable(Action.new(name), KickAction)
	self.lost_ticks = 0
	return self
end

--- How lined up the robot already is to kick the ball towards `target`, in [0, 1]:
--- 1 when it comes at the ball straight along the kick direction, 0 when it has to
--- go all the way around the ball first. Lining up takes time a defender can use.
--- @param robot { x: number, y: number }
--- @param ball { x: number, y: number }
--- @param target { x: number, y: number }
--- @return number
function KickAction.alignment(robot, ball, target)
	local approach = math.atan(ball.y - robot.y, ball.x - robot.x)
	local kick = math.atan(target.y - ball.y, target.x - ball.x)
	return (1.0 + math.cos(kick - approach)) / 2.0
end

--- Seconds the nearest enemy needs to reach the ball (robot profile estimate).
--- @param ball { x: number, y: number }
--- @return number
local function enemy_time_to_ball(ball)
	local best = math.huge
	for _, e in ipairs(world.active_enemies()) do
		best = math.min(best, profile.time_to_reach(e, ball))
	end
	return best
end

--- Runs the kick towards `target` for the robot in `state`. Under pressure (an enemy
--- can reach the ball within PRESSURE_S) and on the wrong side of the ball, it first
--- circles the ball (orbit_ball) instead of the slower stop-and-go approach of
--- kick_to_point; otherwise it kicks directly.
--- @param state table
--- @param target { x: number, y: number }
function KickAction.kick_towards(state, target)
	local robot, ball = state.robot, state.ball or world.ball()
	if enemy_time_to_ball(ball) <= PRESSURE_S and skill_orbit.needed(robot, ball, target) then
		skill_orbit.process(robot.id, robot.team, target)
		return
	end
	skill_kick.process(robot.id, robot.team, target)
end

function KickAction:start(state)
	self.lost_ticks = 0
end

function KickAction:terminated(state)
	if world.ball_speed() > KICKED_SPEED then
		return true
	end
	self.lost_ticks = state.can_kick and 0 or self.lost_ticks + 1
	return self.lost_ticks > LOST_TICKS
end

return KickAction
