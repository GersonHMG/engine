local kick = require("skills.kick_to_point")

local Shoot = { State = { playing = "playing", failed = "failed", done = "done" } }
Shoot.__index = Shoot

function Shoot.new()
    return setmetatable({ state = Shoot.State.playing }, Shoot)
end

--- Executes a shot on goal
--- @param robotId number
--- @param team number
function Shoot:process(robotId, team)
    -- Early exit if already finished or failed
    if self.state ~= Shoot.State.playing then
        return self.state == Shoot.State.done
    end

    -- Hardcoded goal position
    local goal_target = { x = 4.5, y = 0.0 }

    -- Optional: Draw a red dot at the goal target for debugging
    draw_point(goal_target.x, goal_target.y, true, {r=1.0, g=0.0, b=0.0})

    -- Pass the goal coordinate directly to the kick function
    if kick.process(robotId, team, goal_target) then
        self.state = Shoot.State.done
    end

    return self.state == Shoot.State.done
end

function Shoot:get_state()
    return self.state
end

function Shoot:reset()
    self.state = Shoot.State.playing
end

-- Default instance and module export
local def = Shoot.new()

return {
    State = Shoot.State,
    new = Shoot.new,
    process = function(r, t) return def:process(r, t) end,
    get_state = function() return def:get_state() end,
    reset = function() def:reset() end
}