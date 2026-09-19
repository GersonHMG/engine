-- Action.lua
--
-- Abstract base "class" for actions. Concrete actions (KickToGoal,
-- GoToBall, ...) extend this and must override evaluate() and run().
-- Calling the base implementations directly is a programming error.

local Action = {}
Action.__index = Action

--- @param name string  -- unique action name, shown in ACTIONS table / debug
--- @return Action
function Action.new(name)
	local self = setmetatable({}, Action)
	self.name = name
	return self
end

--- Pure function of `state`. Must be overridden.
--- No side effects, no mutation of `state`.
--- @param state table
--- @return number  -- higher = more desirable for the planner to pick
function Action:evaluate(state)
	error(("%s:evaluate() not implemented"):format(self.name or "Action"))
end

--- Side-effecting execution of the action. Must be overridden.
--- @param state table
--- @return nil
function Action:run(state)
	error(("%s:run() not implemented"):format(self.name or "Action"))
end

return Action