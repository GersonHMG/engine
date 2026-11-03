-- PushBall.lua
--
-- Pushes the ball PUSH_STEP forward with the kicker face (skills/push_ball): the dribble of
-- docs/action-scoring.md, section 3.3. Against a defender that stands in front of the ball
-- (the 3v3 ball guard), a shot only bounces off it and the ball moves about 0.3 m per
-- second; a push carries it at PUSH_SPEED while the guard backs off, until the guard reaches
-- the defense area and has to stop. Scored in the shared unit (AI/calculator/value.push_to):
--
--     S = γ^t·[P_keep·V(q) + (1 − P_keep)·V_LOST(q)]
--
-- over the push targets q (value.push_targets). The target is committed when the option
-- starts; the option ends when the ball has gone most of the way, or it lost the ball.
local Action = require("AI.actions.action")
local world = require("AI.calculator.world")
local value = require("AI.calculator.value")
local skill = require("skills.push_ball")

local PushBall = setmetatable({}, { __index = Action })
PushBall.__index = PushBall

local UNAVAILABLE = -1.0  -- no free push target: below any real outcome
local DONE_FRACTION = 0.9 -- the push is done once the ball went this part of the way
local LOST_DIST = 0.25    -- m, robot to ball: farther than this the ball got away
local LOST_TICKS = 30     -- ticks the ball may be away before the push has failed

--- @param team integer
--- @return PushBall
function PushBall.new(team)
	local self = setmetatable(Action.new("push_ball"), PushBall)
	self.team = team
	self.candidate = nil
	self.target = nil
	self.from = nil
	self.lost_ticks = 0
	self.brake = false
	return self
end

function PushBall:evaluate(state)
	self.candidate = nil
	if not state.can_kick then
		return 0.0
	end
	local ball = state.ball or world.ball()
	local best = value.best_push(state.robot, ball, state.opponents or world.active_enemies())
	if not best then
		return UNAVAILABLE
	end
	self.candidate = best.target
	return best.score
end

function PushBall:start(state)
	self.target = self.candidate
	local ball = state.ball or world.ball()
	self.from = { x = ball.x, y = ball.y }
	self.lost_ticks = 0
	-- Brake when the ball must not roll on past the target (into an area, out of the field).
	self.brake = self.target ~= nil and not value.push_rolls_on(self.from, self.target)
end

function PushBall:terminated(state)
	if not self.target or not self.from then return true end
	local ball = state.ball or world.ball()
	local total = world.distance(self.from, self.target)
	if world.distance(self.from, ball) >= DONE_FRACTION * total then
		return true
	end
	self.lost_ticks = world.distance(state.robot, ball) > LOST_DIST and self.lost_ticks + 1 or 0
	return self.lost_ticks > LOST_TICKS
end

function PushBall:run(state)
	local target = self.target or self.candidate
	if not target then return end
	skill.process(state.robot.id, state.robot.team, target, nil, self.brake)
end

return PushBall
