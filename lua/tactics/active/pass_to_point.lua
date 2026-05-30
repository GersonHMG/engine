local kick = require("skills.kick_to_point")

local PassToPoint = { State = { playing = "PLAYING", failed = "FAILED", done = "DONE" } }
PassToPoint.__index = PassToPoint

function PassToPoint.new()
    return setmetatable({ state = PassToPoint.State.playing }, PassToPoint)
end

--- Executes a pass to a specific coordinate
--- @param robotId number
--- @param team number
--- @param targetPoint table - A table containing x and y coordinates (e.g., {x = 1.0, y = 2.5})
function PassToPoint:process(robotId, team, targetPoint)
    -- Early exit if already finished or failed
    if self.state ~= PassToPoint.State.playing then
        return self.state == PassToPoint.State.done
    end

    -- Safety check: Ensure a valid point was passed
    if type(targetPoint) ~= "table" or not targetPoint.x or not targetPoint.y then
        self.state = PassToPoint.State.failed
        return false
    end

    -- Pass coordinate directly to kick function
    if kick.process(robotId, team, targetPoint) then
        self.state = PassToPoint.State.done
    end

    return self.state == PassToPoint.State.done
end

function PassToPoint:get_state()
    return self.state
end

function PassToPoint:reset()
    self.state = PassToPoint.State.playing
end

-- Default instance and module export
local def = PassToPoint.new()

return {
    State = PassToPoint.State,
    new = PassToPoint.new,
    process = function(r, t, tp) return def:process(r, t, tp) end,
    get_state = function() return def:get_state() end,
    reset = function() def:reset() end
}