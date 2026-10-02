-- Pass.lua
local KickAction = require("AI.actions.kick_action")
local world = require("AI.calculator.world")
local lane = require("AI.calculator.lane")
local skill_kick = require("skills.kick_to_point")

local Pass = setmetatable({}, { __index = KickAction })
Pass.__index = Pass

local GOAL_TARGET = { x = 4.5, y = 0.0 }  -- enemy goal center
local GAMMA = 0.9                         -- discount: a pass is one more step before the shot
local MIN_PASS_DIST = 1.0                 -- a mate closer than this is not worth passing to

--- Value of passing to `mate`: the ball has to get there, then the mate shoots.
---     V(j) = Λ(b, r_j, E) · P_s(r_j)
local function pass_value(ball, mate, opponents)
	if world.distance(ball, mate) < MIN_PASS_DIST then
		return 0.0
	end
	return lane.clear(ball, mate, opponents) * world.probability_of_score(mate, GOAL_TARGET)
end

--- @param team integer
--- @return Pass
function Pass.new(team)
	local self = setmetatable(KickAction.new("pass"), Pass)
	self.team = team
	self.candidate_id = nil  -- best receiver this tick, from evaluate()
	self.receiver_id = nil   -- receiver committed when the option started
	return self
end

--- S = k · γ · max_j V(j), over the teammates in `state.teammates`.
function Pass:evaluate(state)
	self.candidate_id = nil
	if not state.can_kick then
		return 0.0
	end

	local opponents = state.opponents or world.active_enemies()
	local best = 0.0
	for _, mate in ipairs(state.teammates or {}) do
		local v = pass_value(state.ball, mate, opponents)
		if v > best then
			best, self.candidate_id = v, mate.id
		end
	end

	return GAMMA * best
end

function Pass:start(state)
	KickAction.start(self, state)
	self.receiver_id = self.candidate_id
end

--- Kicks at the committed receiver's current position. The receiver faces
--- the ball and moves onto its path once it is in flight (see ReceivePass).
function Pass:run(state)
	local mate = self.receiver_id and world.ally(self.receiver_id)
	if not mate then return end
	skill_kick.process(state.robot.id, state.robot.team, { x = mate.x, y = mate.y })
end

return Pass
