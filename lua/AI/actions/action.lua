-- Action.lua
--
-- Abstract base "class" for actions. Concrete actions (KickToGoal,
-- GoToBall, ...) extend this and must override evaluate() and run().
-- Calling the base implementations directly is a programming error.
--
-- Each action is an option (Sutton, Precup & Singh, 1999):
--   evaluate   -- score; > 0 means the option may start (initiation set)
--   start      -- called once when the role switches to this option
--   run        -- intra-option policy, called every tick while it runs
--   terminated -- termination condition β, checked every tick while it runs
--
-- The defaults (start does nothing, terminated always true) make a one-step
-- option: the role re-decides it every tick.

local Action = {}
Action.__index = Action

--- @param name string  -- unique action name, shown in ACTIONS table / debug
--- @return Action
function Action.new(name)
	local self = setmetatable({}, Action)
	self.name = name
	return self
end

--- Must be overridden. Must not command robots or mutate `state`; it may
--- remember what it scored (e.g. a candidate target) for start() to commit.
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

--- Called once when the option starts, before its first run().
--- @param state table
function Action:start(state)
end

--- Termination condition β: true once the option has finished or failed.
--- Called once per tick, only while the option is running.
--- @param state table
--- @return boolean
function Action:terminated(state)
	return true
end

return Action
