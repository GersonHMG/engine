-- utils/blackboard.lua
local Blackboard = {
    _data = {}
}

--- Writes a value to the shared memory
--- @param key string - The name of the variable you are saving
--- @param value any - The data you want to save
function Blackboard.set(key, value)
    Blackboard._data[key] = value
end

--- Reads a value from the shared memory
--- @param key string
--- @return any
function Blackboard.get(key)
    return Blackboard._data[key]
end

--- Clears the memory (Useful to call at the start or end of a match)
function Blackboard.clear()
    Blackboard._data = {}
end

return Blackboard