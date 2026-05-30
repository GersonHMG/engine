local kick = require("skills.kick_to_point")

local Pass = { State = { playing = "PLAYING", failed = "FAILED", done = "DONE" } }
Pass.__index = Pass

function Pass.new()
    return setmetatable({ state = Pass.State.playing }, Pass)
end

function Pass:process(robotId, team, targetRobotId)
    -- Early exit if already finished or failed
    if self.state ~= Pass.State.playing then
        return self.state == Pass.State.done
    end

    local mate = get_robot_state(targetRobotId, team)
    if not mate then
        self.state = Pass.State.failed
        return false
    end

    -- Pass coordinate directly to kick function
    if kick.process(robotId, team, { x = mate.x, y = mate.y }) then
        self.state = Pass.State.done
    end

    return self.state == Pass.State.done
end

function Pass:get_state()
    return self.state
end

function Pass:reset()
    self.state = Pass.State.playing
end

-- Default instance and module export
local def = Pass.new()

return {
    State = Pass.State,
    new = Pass.new,
    process = function(r, t, tr) return def:process(r, t, tr) end,
    get_state = function() return def:get_state() end,
    reset = function() def:reset() end
}