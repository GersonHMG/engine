-- KickToClear.lua
local Action = require("AI.actions.action")
local world = require("AI.calculator.world")
local skill_kick = require("skills.kick_to_point")

local KickToClear = setmetatable({}, { __index = Action })
KickToClear.__index = KickToClear

local CLEAR_DIST      = 2.0   -- how far from the ball the clear target sits, meters
local CANDIDATES      = 16    -- directions sampled around the ball
local ROBOT_RADIUS    = 0.09
local BALL_RADIUS     = 0.021
local BLOCK_SHARPNESS = 12.0  -- logistic steepness for lane clearance

local function dist(a, b)
	return math.sqrt((a.x - b.x) ^ 2 + (a.y - b.y) ^ 2)
end

--- Perpendicular distance from point c to segment a->b.
local function point_to_segment(c, a, b)
	local dx, dy = b.x - a.x, b.y - a.y
	local len2 = dx * dx + dy * dy
	if len2 < 1e-9 then return dist(c, a) end
	local t = ((c.x - a.x) * dx + (c.y - a.y) * dy) / len2
	t = math.max(0.0, math.min(1.0, t))
	return dist(c, { x = a.x + t * dx, y = a.y + t * dy })
end

--- P(ball travels a->b without an opponent intercepting it), in [0, 1].
local function lane_clear(a, b, opponents)
	local p = 1.0
	local clearance = ROBOT_RADIUS + BALL_RADIUS
	for _, opp in ipairs(opponents) do
		local d = point_to_segment(opp, a, b)
		p = p / (1.0 + math.exp(-BLOCK_SHARPNESS * (d - clearance)))
	end
	return p
end

--- Picks the most open direction around the ball. Directions the robot is
--- already lined up for (robot -> ball) are preferred, so it barely has to
--- reposition before kicking.
local function pick_target(robot, ball, opponents)
	local approach = math.atan(ball.y - robot.y, ball.x - robot.x)
	local best, best_score = nil, 0.0

	for i = 0, CANDIDATES - 1 do
		local a = 2 * math.pi * i / CANDIDATES
		local p = { x = ball.x + CLEAR_DIST * math.cos(a), y = ball.y + CLEAR_DIST * math.sin(a) }
		local alignment = (1.0 + math.cos(a - approach)) / 2.0
		local s = lane_clear(ball, p, opponents) * (0.5 + 0.5 * alignment)
		if s > best_score then
			best, best_score = p, s
		end
	end

	return best
end

--- @param team integer
--- @return KickToClear
function KickToClear.new(team)
	local self = setmetatable(Action.new("kick_to_clear"), KickToClear)
	self.team = team
	self.target = nil
	return self
end

--- Needs possession. The target is chosen once and kept until possession is
--- lost, so the robot's own movement cannot keep moving it.
function KickToClear:evaluate(state)
	if not state.can_kick then
		self.target = nil
		return 0.0
	end

	local ball = state.ball or world.ball()
	local opponents = state.opponents or world.active_enemies()

	if not self.target then
		self.target = pick_target(state.robot, ball, opponents)
		if not self.target then return 0.0 end
	end

	return lane_clear(ball, self.target, opponents)
end

function KickToClear:run(state)
	if not self.target then return end
	skill_kick.process(state.robot.id, state.robot.team, self.target)
end

return KickToClear
