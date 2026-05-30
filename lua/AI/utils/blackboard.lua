local Blackboard = {}
Blackboard.__index = Blackboard

--- Creates a brand new, isolated memory board
function Blackboard.new()
    local self = setmetatable({}, Blackboard)
    self._data = {}
    return self
end

function Blackboard:set(key, value)
    self._data[key] = value
end

function Blackboard:get(key)
    return self._data[key]
end

function Blackboard:clear()
    self._data = {}
end

return Blackboard