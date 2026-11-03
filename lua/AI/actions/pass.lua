-- Pass.lua
--
-- Passes to a teammate. Scored in the shared unit of docs/action-scoring.md (section 3.2)
-- by AI/calculator/value.pass_to: the pass has to be kicked before an enemy takes the
-- ball, get through the lane and be controlled; then the receiver holds the ball where it
-- stands, worth max(V(p), its best shot from there). Teammates inside a defense area (our
-- goalkeeper) are not passed to. The receiver is committed when the option starts.
local KickAction = require("AI.actions.kick_action")
local world = require("AI.calculator.world")
local value = require("AI.calculator.value")
local area_guard = require("utils.area_guard")
local profile = require("utils.robot_profile")

local Pass = setmetatable({}, { __index = KickAction })
Pass.__index = Pass

local UNAVAILABLE = -1.0  -- score with nobody to pass to: below any real outcome

--- @param team integer
--- @return Pass
function Pass.new(team)
	local self = setmetatable(KickAction.new("pass"), Pass)
	self.team = team
	self.candidate_id = nil  -- best receiver this tick, from evaluate()
	self.receiver_id = nil   -- receiver committed when the option started
	return self
end

--- S = max over teammates of value.pass_to.
function Pass:evaluate(state)
	self.candidate_id = nil
	if not state.can_kick then
		return 0.0
	end
	local ball = state.ball or world.ball()
	local opponents = state.opponents or world.active_enemies()
	local best = nil
	for _, mate in ipairs(state.teammates or {}) do
		if not area_guard.touched(mate, profile.robot.radius) then
			local option = value.pass_to(state.robot, ball, mate, opponents)
			if option and (not best or option.score > best.score) then
				best = option
			end
		end
	end
	if not best then
		return UNAVAILABLE
	end
	self.candidate_id = best.receiver
	return best.score
end

function Pass:start(state)
	KickAction.start(self, state)
	self.receiver_id = self.candidate_id
	local mate = self.receiver_id and world.ally(self.receiver_id)
	self.point = mate and { x = mate.x, y = mate.y } or nil
end

--- Kicks at the point where the receiver stood when the pass started: a target that moves
--- with the receiver can never be lined up within the kick's tolerance. The receiver faces
--- the ball and moves onto its path once it is in flight (see ReceivePass).
function Pass:run(state)
	if not self.point then return end
	KickAction.kick_towards(state, self.point)
end

return Pass
