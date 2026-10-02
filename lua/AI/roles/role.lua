-- Role.lua
--
-- A role is the set of actions a robot may pick from. Each robot gets its
-- own Role instance (with its own action instances), so per-action memory
-- such as a chosen kick target never leaks between robots.
--
-- Selection follows the options framework: an action, once started, runs
-- until its termination condition β fires (Action:terminated). While it
-- runs it is only preempted by a challenger that beats it by more than
-- DELIBERATION_COST (Harb et al., 2018):
--
--     o_t = o_{t-1}          if not β(s_t) and S(o_{t-1}) ≥ max_o S(o) − DELIBERATION_COST
--           argmax_o S(o)    otherwise
--
-- A running option whose score dropped to 0 (briefly outside its initiation
-- set, e.g. the ball slipped out of kick range) is not preempted: β alone
-- decides whether it has failed.

local Role = {}
Role.__index = Role

Role.DELIBERATION_COST = 0.05

--- @param name string
--- @param actions Action[]
--- @param override? fun(state: table, scored: table[]): integer|nil  -- forces an index when it returns one
--- @return Role
function Role.new(name, actions, override)
	local self = setmetatable({}, Role)
	self.name = name
	self.actions = actions
	self.override = override
	self.current = nil  -- index of the running option
	return self
end

--- Scores every action for `state` and picks the one to run.
--- @param state table
--- @return { action: Action, score: number }[] scored  -- same order as actions
--- @return integer best_index
function Role:choose(state)
	local scored = {}
	local best = 1

	for i, action in ipairs(self.actions) do
		scored[i] = { action = action, score = action:evaluate(state) }
		if scored[i].score > scored[best].score then
			best = i
		end
	end

	local current = self.current
	local running = current ~= nil and not self.actions[current]:terminated(state)
	if running then
		local current_score = scored[current].score
		if current_score == 0.0 or scored[best].score - current_score <= Role.DELIBERATION_COST then
			best = current
		end
	end

	if self.override then
		best = self.override(state, scored) or best
	end

	-- A new option starts when the old one ended, even if it is picked again.
	if not running or best ~= current then
		self.actions[best]:start(state)
	end

	self.current = best
	return scored, best
end

--- Drops the running option, called when the robot switches into this role.
function Role:reset()
	self.current = nil
end

return Role
