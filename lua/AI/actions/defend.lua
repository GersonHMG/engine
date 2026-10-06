-- Defend.lua
-- A defender's action: mark the enemy it was given (state.mark_id, skills/mark: between
-- that enemy and the ball), or, with nobody to mark, guard the ball: stand on the line
-- ball -> our goal, GUARD_RADIUS from the goal center (outside our defense area); guards
-- after the first spread sideways by GUARD_SPACING.
local Action = require("AI.actions.action")
local mark = require("skills.mark")

local Defend = setmetatable({}, { __index = Action })
Defend.__index = Defend

local OWN_GOAL = { x = -4.5, y = 0.0 }  -- our goal center
local GUARD_RADIUS = 1.6    -- m from our goal center: outside the defense area (1 m deep, 2 m wide)
local GUARD_SPACING = 0.25  -- m between guards side by side

--- @param team integer
--- @return Defend
function Defend.new(team)
	local self = setmetatable(Action.new("defend"), Defend)
	self.team = team
	return self
end

function Defend:evaluate(state)
	return 1.0
end

--- The guard spot for the `index`-th guard (1, 2, ...): on the ball -> goal line, then
--- alternately to one side and the other of it.
function Defend.guard_spot(ball, index)
	local dx, dy = ball.x - OWN_GOAL.x, ball.y - OWN_GOAL.y
	local length = math.max(math.sqrt(dx * dx + dy * dy), 1e-3)
	local ux, uy = dx / length, dy / length
	local k = index - 1
	local side = (k % 2 == 1) and 1 or -1
	local offset = side * math.ceil(k / 2) * GUARD_SPACING
	return {
		x = OWN_GOAL.x + ux * GUARD_RADIUS - uy * offset,
		y = OWN_GOAL.y + uy * GUARD_RADIUS + ux * offset,
	}
end

function Defend:run(state)
	local robot, ball = state.robot, state.ball
	if state.mark_id then
		mark.process(robot.id, robot.team, state.mark_id)
		return
	end
	local spot = Defend.guard_spot(ball, state.guard_index or 1)
	draw_point(spot.x, spot.y, true, { r = 0.6, g = 0.6, b = 1.0 })
	move_to(robot.id, robot.team, spot)
	face_to(robot.id, robot.team, { x = ball.x, y = ball.y })
end

return Defend
