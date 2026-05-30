local m = {}

local SupportClass = require("AI.support")
local OffenseClass = require("AI.offense")
local team_manager = require("AI.team_manager")
local blackboard = require("AI.blackboard") -- Import the shared memory

grsim.teleport_robot(0, 0, -0.5, -0.5, -0.0)
grsim.teleport_robot(1, 0, 1.0, 1.0, -9.0)
grsim.teleport_ball(-0.35, -0.5)

-- Initialize the manager (Team 0, Robot ID 0, Robot ID 1)
local manager = team_manager.new(0, 0, 1)

-- 1. Create our active role instances
local active_offense = OffenseClass.new()
local active_support = SupportClass.new()

-- We will store who had the ball last frame to detect changes
local previous_offense_id = nil

function m.process()
    -- Ask the manager who should do what
    local offense_id, support_id = manager:get_roles()

    -- 2. Detect if a role swap just happened!
    if previous_offense_id ~= nil and previous_offense_id ~= offense_id then
        print("Roles swapped! Instantiating fresh role objects.")
        
        -- Trash the old objects and create brand new ones from scratch
        active_offense = OffenseClass.new()
        active_support = SupportClass.new()
        
        -- WIPE THE BLACKBOARD:
        -- Prevent the new Offense robot from passing to an old, stale coordinate.
        if blackboard.clear then 
            blackboard.clear()
        end
    end

    -- Update our tracker for the next frame
    previous_offense_id = offense_id

    -- 3. Execute the roles
    -- CRITICAL: Support MUST run first so it can calculate and post the 
    -- 'best_pass_point' to the blackboard before the Offense robot reads it.
    active_support:process(support_id, 0)
    
    -- Offense runs second, reads the blackboard, and executes the play
    active_offense:process(offense_id, 0, support_id)
end

return m