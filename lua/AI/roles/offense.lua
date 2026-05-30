local pass_to_point = require("tactics.active.pass_to_point")
local shoot_module = require("tactics.active.shoot")
local utils = require("utils.utils")

local Offense = {}
Offense.__index = Offense

function Offense.new(team_blackboard)
    local self = setmetatable({ current_action = nil }, Offense)
    self.blackboard = team_blackboard
    return self
end

--- Executes the Offense role decision engine
--- @param robotId number
--- @param team number
--- @param teammateId number - (Optional now, but kept for compatibility)
function Offense:process(robotId, team, teammateId)
    local robot = get_robot_state(robotId, team)

    if not robot then
        return
    end
    
    local action_str = self.current_action or "evaluating"
    local display_text = "Offense (" .. action_str .. ")"
    draw_text(robot.x, robot.y + 0.2, display_text, {r=1.0, g=0.0, b=0.0})

    -- 1. Define the goal position and our shooting threshold
    local goal_target = { x = 4.5, y = 0.0 }
    local dist_to_goal = utils.getDistance(robot, goal_target)
    
    -- If the robot is closer than 2.5 meters to the goal, it will shoot.
    -- Otherwise, it will look for a pass. Adjust this based on your field size!
    local SHOOT_DISTANCE = 1.5 

    -- 2. If we aren't currently locked into an action, make a decision
    if not self.current_action then
        if dist_to_goal <= SHOOT_DISTANCE then
            self.current_action = "shooting"
            shoot_module.reset()
        else
            self.current_action = "passing"
            pass_to_point.reset()
        end
    end
    
    -- 3. Execute the chosen action
    if self.current_action == "shooting" then
        local is_done = shoot_module.process(robotId, team)
        
        -- If the shot is complete or failed, clear the action lock to decide again
        if is_done or shoot_module.get_state() == shoot_module.State.failed then
            self.current_action = nil
        end

    elseif self.current_action == "passing" then
        -- Read the best open coordinate from the shared memory
        local pass_target = self.blackboard:get("best_pass_point")
        -- Make sure the Support role has actually posted a point before passing
        if pass_target then
            local is_done = pass_to_point.process(robotId, team, pass_target)
            
            -- If the pass is complete, transition to the stop state
            if is_done then
                self.current_action = "stopped"
                
            -- If it failed, clear the lock so it can evaluate the field again
            elseif pass_to_point.get_state() == pass_to_point.State.failed then
                self.current_action = nil
            end
        else
            -- If the blackboard is empty, it just waits until a point is available
        end

    elseif self.current_action == "stopped" then
        
        -- 4. The robot has successfully passed and is now stopped
        -- If your API has a specific stop command, you can place it here.
        -- Otherwise, doing nothing will let it hold its position.
        
        -- Example (if supported by your API):
        -- stop_robot(robotId, team)
    end
end

-- Default instance and module export
local def = Offense.new()

return {
    new = Offense.new,
    process = function(r, t, teammateId) def:process(r, t, teammateId) end
}