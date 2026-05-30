-- Decide when to switch roles

local utils = require("utils.utils")

local TeamManager = {}
TeamManager.__index = TeamManager

function TeamManager.new(team, robot1_id, robot2_id)
    local self = setmetatable({}, TeamManager)
    self.team = team
    self.r1 = robot1_id
    self.r2 = robot2_id
    self.current_offense_id = nil
    
    return self
end

--- Calculates which robot should play offense and support
--- @return number, number - Returns (offense_id, support_id)
function TeamManager:get_roles()
    local ball = get_ball_state()
    local robot1 = get_robot_state(self.r1, self.team)
    local robot2 = get_robot_state(self.r2, self.team)

    -- Fallback: If vision is temporarily lost, just return the last known assignments
    if not ball or not robot1 or not robot2 then
        local off_id = self.current_offense_id or self.r1
        local supp_id = (off_id == self.r1) and self.r2 or self.r1
        return off_id, supp_id
    end

    local dist1 = utils.getDistance(robot1, ball)
    local dist2 = utils.getDistance(robot2, ball)
    local offense_id, support_id
    
    -- Hysteresis (Anti-Jitter)
    local SWAP_THRESHOLD = 0.2 
    
    if self.current_offense_id == self.r1 then
        if dist2 < (dist1 - SWAP_THRESHOLD) then
            offense_id, support_id = self.r2, self.r1
        else
            offense_id, support_id = self.r1, self.r2
        end
    elseif self.current_offense_id == self.r2 then
        if dist1 < (dist2 - SWAP_THRESHOLD) then
            offense_id, support_id = self.r1, self.r2
        else
            offense_id, support_id = self.r2, self.r1
        end
    else
        -- Initial state
        if dist1 < dist2 then
            offense_id, support_id = self.r1, self.r2
        else
            offense_id, support_id = self.r2, self.r1
        end
    end

    -- Update our tracker and return the IDs
    self.current_offense_id = offense_id
    return offense_id, support_id
end

return {
    new = TeamManager.new
}