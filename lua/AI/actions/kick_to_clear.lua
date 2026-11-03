-- KickToClear.lua
--
-- Kicks the ball into space: forward to a better place to play from, or away from our goal
-- when we are under pressure there. Scored in the shared unit of docs/action-scoring.md
-- (section 3.4), over DIRECTIONS kick directions (AI/calculator/value):
--
--     S = P_rel·γ^t·[P_win·V(p) + (1 − P_win)·V_LOST(p)] + (1 − P_rel)·γ^{t_kick}·V_LOST(b)
--
-- p is where the kick stops rolling. Directions that leave the field, stop in a defense
-- area or need a kicking spot inside one are not tried. The target is committed when the
-- option starts.
local KickAction = require("AI.actions.kick_action")
local world = require("AI.calculator.world")
local value = require("AI.calculator.value")

local KickToClear = setmetatable({}, { __index = KickAction })
KickToClear.__index = KickToClear

local UNAVAILABLE = -1.0  -- score when no direction can be kicked: below any real outcome

--- @param team integer
--- @return KickToClear
function KickToClear.new(team)
	local self = setmetatable(KickAction.new("kick_to_clear"), KickToClear)
	self.team = team
	self.candidate = nil  -- best target this tick, from evaluate()
	self.target = nil     -- target committed when the option started
	return self
end

function KickToClear:evaluate(state)
	self.candidate = nil
	if not state.can_kick then
		return 0.0
	end
	local ball = state.ball or world.ball()
	local opponents = state.opponents or world.active_enemies()
	local best = value.best_kick_into_space(state.robot, ball, state.teammates or {}, opponents)
	if not best then
		return UNAVAILABLE
	end
	self.candidate = best.target
	return best.score
end

function KickToClear:start(state)
	KickAction.start(self, state)
	self.target = self.candidate
end

function KickToClear:run(state)
	local target = self.target or self.candidate
	if not target then return end
	KickAction.kick_towards(state, target)
end

return KickToClear
