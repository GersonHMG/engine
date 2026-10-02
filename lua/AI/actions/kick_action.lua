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

local KickAction = setmetatable({}, { __index = Action })
KickAction.__index = KickAction

local KICKED_SPEED = 1.0  -- m/s, faster than this the ball was kicked
local LOST_TICKS = 30     -- ticks out of kick range before giving up (≈0.5 s at 60 Hz)

--- @param name string
--- @return KickAction
function KickAction.new(name)
	local self = setmetatable(Action.new(name), KickAction)
	self.lost_ticks = 0
	return self
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
