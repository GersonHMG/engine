-- Pass.lua
--
-- Passes to a teammate. Scored in the shared unit of docs/action-scoring.md (section 3.2)
-- by AI/calculator/value.pass_to: the pass has to be kicked before an enemy takes the
-- ball, get through the lane and be controlled; then the receiver holds the ball where it
-- is received, worth max(V(p), its best shot from there).
--
-- Lead passes: the target is a point p the receiver runs to (LEAD_RADII around where it
-- stands, LEAD_DIRECTIONS directions), with t_ready its time to get there. A marker stands
-- in the lane to where the receiver stands now; a point beside it opens the lane. Lanes a
-- robot already covers are not offered (value.pass_to).
--
-- The plan is shared: while the option runs, Pass.planned[receiver id] holds the point, so
-- the receiver (PositionForPass) runs there instead of to its own spot. The passer kicks as
-- soon as it is lined up: waiting for the receiver gives a marker that follows it the time
-- to step back into the lane. Once the ball is in flight the receiver steps onto its path
-- (ReceivePass).
local KickAction = require("AI.actions.kick_action")
local world = require("AI.calculator.world")
local value = require("AI.calculator.value")
local area_guard = require("utils.area_guard")
local profile = require("utils.robot_profile")

local Pass = setmetatable({}, { __index = KickAction })
Pass.__index = Pass

local UNAVAILABLE = -1.0  -- score with nobody to pass to: below any real outcome
local LEAD_RADII = { 0.0, 0.6, 1.2, 1.8 }  -- m, how far from the receiver a pass may be led: the marker
                                          -- follows it, so the run has to outpace the marker into the lane
local LEAD_DIRECTIONS = 8
local FIELD_ROOM = 0.3    -- m, lead points stay this far inside the field lines
local PLAN_TICKS = 3      -- ticks a plan stays valid without being refreshed

--- Planned passes, by receiver id: { point = {x, y}, tick = world tick it was refreshed }.
Pass.planned = {}

--- The point a pass to robot `id` is heading for, or nil.
--- @param id integer
--- @return { x: number, y: number }|nil
function Pass.plan_for(id)
	local plan = Pass.planned[id]
	if plan and world.tick - plan.tick <= PLAN_TICKS then return plan.point end
	return nil
end

--- @param team integer
--- @return Pass
function Pass.new(team)
	local self = setmetatable(KickAction.new("pass"), Pass)
	self.team = team
	self.candidate = nil   -- best { id, point } this tick, from evaluate()
	self.receiver_id = nil -- receiver committed when the option started
	self.point = nil       -- point committed when the option started
	return self
end

--- Points a pass to `mate` may be led to: where it stands, and LEAD_RADII around it,
--- inside the field and out of the defense areas.
local function lead_points(mate)
	local out = {}
	for _, r in ipairs(LEAD_RADII) do
		local n = r == 0 and 1 or LEAD_DIRECTIONS
		for k = 0, n - 1 do
			local a = 2 * math.pi * k / n
			local p = { x = mate.x + r * math.cos(a), y = mate.y + r * math.sin(a) }
			if math.abs(p.x) <= value.FIELD_HALF_X - FIELD_ROOM and math.abs(p.y) <= value.FIELD_HALF_Y - FIELD_ROOM
				and not area_guard.touched(p, area_guard.MARGIN) then
				out[#out + 1] = p
			end
		end
	end
	return out
end

--- S = max over teammates and their lead points of value.pass_to.
function Pass:evaluate(state)
	self.candidate = nil
	if not state.can_kick then
		return 0.0
	end
	local ball = state.ball or world.ball()
	local opponents = state.opponents or world.active_enemies()
	local best, best_c = nil, nil
	for _, mate in ipairs(state.teammates or {}) do
		if not area_guard.touched(mate, profile.robot.radius) then
			for _, p in ipairs(lead_points(mate)) do
				local at = { id = mate.id, x = p.x, y = p.y, orientation = mate.orientation }
				local t_ready = profile.time_to_reach(mate, p)
				local option = value.pass_to(state.robot, ball, at, opponents, t_ready, mate)
				if option and (not best or option.score > best.score) then
					best, best_c = option, { id = mate.id, point = p }
				end
			end
		end
	end
	if not best then
		return UNAVAILABLE
	end
	self.candidate = best_c
	return best.score
end

function Pass:start(state)
	KickAction.start(self, state)
	self.receiver_id = self.candidate and self.candidate.id or nil
	self.point = self.candidate and self.candidate.point or nil
end

--- Kicks at the committed point as soon as it is lined up (a moving target never lines up).
function Pass:run(state)
	if not self.point then return end
	Pass.planned[self.receiver_id] = { point = self.point, tick = world.tick }
	KickAction.kick_towards(state, self.point)
end

return Pass
