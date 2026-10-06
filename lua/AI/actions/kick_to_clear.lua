-- KickToClear.lua
local KickAction = require("AI.actions.kick_action")
local world = require("AI.calculator.world")
local lane = require("AI.calculator.lane")
local profile = require("utils.robot_profile")

local KickToClear = setmetatable({}, { __index = KickAction })
KickToClear.__index = KickToClear

local OWN_GOAL   = { x = -4.5, y = 0.0 }  -- our own goal center
local CANDIDATES = 16    -- directions sampled around the ball
local FIELD_HALF_X, FIELD_HALF_Y = 4.5, 3.0  -- m
local FIELD_MARGIN = 0.2 -- m, the ball must stop at least this far inside the field
local LAMBDA     = 0.35  -- decay of urgency with distance to our own goal

--- Picks the most open direction around the ball. Directions the robot is
--- already lined up for (robot -> ball) are preferred, so it barely has to
--- reposition before kicking.
local function pick_target(robot, ball, opponents)
	local approach = math.atan(ball.y - robot.y, ball.x - robot.x)
	local best, best_score = nil, 0.0
	-- The clear target is where a kick stops rolling, so the lane check covers the whole roll.
	local clear_dist = profile.roll_distance()

	for i = 0, CANDIDATES - 1 do
		local a = 2 * math.pi * i / CANDIDATES
		local p = { x = ball.x + clear_dist * math.cos(a), y = ball.y + clear_dist * math.sin(a) }
		local alignment = (1.0 + math.cos(a - approach)) / 2.0
		-- A clear that rolls out of the field gives the ball away.
		local inside = math.abs(p.x) <= FIELD_HALF_X - FIELD_MARGIN and math.abs(p.y) <= FIELD_HALF_Y - FIELD_MARGIN
		local s = inside and lane.clear(ball, p, opponents) * (0.5 + 0.5 * alignment) or 0.0
		if s > best_score then
			best, best_score = p, s
		end
	end

	return best
end

--- @param team integer
--- @return KickToClear
function KickToClear.new(team)
	local self = setmetatable(KickAction.new("kick_to_clear"), KickToClear)
	self.team = team
	self.candidate = nil  -- best target this tick, from evaluate()
	self.target = nil     -- target committed when the option started
	return self
end

--- Needs possession. Scores a fresh candidate target every tick; start()
--- commits it, so the robot's own movement cannot move the target mid-kick.
---
--- S = k · Λ(b, target, E) · e^(−λ·d(r, O)): clearing is a defensive action,
--- only urgent near our own goal. Without the urgency term an open lane
--- (Λ ≈ 1) would beat passing and shooting everywhere on the field.
function KickToClear:evaluate(state)
	self.candidate = nil
	if not state.can_kick then
		return 0.0
	end

	local ball = state.ball or world.ball()
	local opponents = state.opponents or world.active_enemies()

	self.candidate = pick_target(state.robot, ball, opponents)
	if not self.candidate then
		return 0.0
	end

	local urgency = math.exp(-LAMBDA * world.distance(state.robot, OWN_GOAL))
	return lane.clear(ball, self.candidate, opponents) * urgency
end

function KickToClear:start(state)
	KickAction.start(self, state)
	self.target = self.candidate
end

function KickToClear:run(state)
	if not self.target then return end
	KickAction.kick_towards(state, self.target)
end

return KickToClear
