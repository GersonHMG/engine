-- KickToGoal.lua
--
-- Shoots at the goal. Scored in the shared unit of docs/action-scoring.md (section 3.1):
-- the probability that the shot ends in a goal, discounted by the time it takes,
--
--     S = γ^{t_kick + flight} · P_rel · P_target · P_clear · P_keeper
--
-- maximized over aim points along the goal mouth (AI/calculator/value.best_shot), so it
-- shoots past the keeper instead of at the goal center. Unavailable (UNAVAILABLE) when the
-- goal is beyond a kick's roll. The aim is committed when the option starts.
local KickAction = require("AI.actions.kick_action")
local world = require("AI.calculator.world")
local value = require("AI.calculator.value")

local KickToGoal = setmetatable({}, { __index = KickAction })
KickToGoal.__index = KickToGoal

local UNAVAILABLE = -1.0  -- score when no aim can be reached: below any real outcome
-- m: once the ball has moved this far since the aim was committed, the aim is stale and is
-- committed again (a ball still rolling after a push: 0.1-0.3 m moved -> the committed aim's
-- P at the kick was 0.50 against 0.69 for the best aim then; run 04 aim probe).
local REAIM_DIST = 0.1

--- @param team integer
--- @return KickToGoal
function KickToGoal.new(team)
	local self = setmetatable(KickAction.new("kick_to_goal"), KickToGoal)
	self.team = team
	self.candidate = nil  -- best aim this tick, from evaluate()
	self.aim = nil        -- aim committed when the option started
	self.aim_ball = nil   -- where the ball was when the aim was committed
	return self
end

function KickToGoal:evaluate(state)
	self.candidate = nil
	if not state.can_kick then
		return 0.0
	end
	local ball = state.ball or world.ball()
	local opponents = state.opponents or world.active_enemies()
	local shot = value.best_shot(state.robot, ball, opponents)
	if not shot then
		return UNAVAILABLE
	end
	self.candidate = shot.aim
	return shot.score
end

function KickToGoal:start(state)
	KickAction.start(self, state)
	self.aim = self.candidate
	local ball = state.ball or world.ball()
	self.aim_ball = { x = ball.x, y = ball.y }
end

function KickToGoal:run(state)
	local ball = state.ball or world.ball()
	if self.candidate and self.aim_ball and world.distance(ball, self.aim_ball) > REAIM_DIST then
		self.aim = self.candidate
		self.aim_ball = { x = ball.x, y = ball.y }
	end
	local aim = self.aim or self.candidate or value.GOAL
	KickAction.kick_towards(state, aim)
end

return KickToGoal
