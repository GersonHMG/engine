local SupportClass = require("AI.roles.support")
local OffenseClass = require("AI.roles.offense")
local TeamManager = require("AI.roles.team_manager")
local BlackboardClass = require("AI.utils.blackboard") -- Require the class here!

local TeamBehavior = {}
TeamBehavior.__index = TeamBehavior

function TeamBehavior.new(team_id, robot1_id, robot2_id)
    local self = setmetatable({}, TeamBehavior)
    
    self.team_id = team_id
    self.manager = TeamManager.new(team_id, robot1_id, robot2_id)
    
    -- 1. Create a private blackboard exclusively for this team
    self.blackboard = BlackboardClass.new()
    
    -- 2. Hand the private blackboard to the roles when creating them
    self.active_offense = OffenseClass.new(self.blackboard)
    self.active_support = SupportClass.new(self.blackboard)
    
    self.previous_offense_id = nil
    
    return self
end

function TeamBehavior:process()
    local offense_id, support_id = self.manager:get_roles()

    if self.previous_offense_id ~= nil and self.previous_offense_id ~= offense_id then
        print("Team " .. self.team_id .. " roles swapped! Resetting.")
        
        -- Make sure to pass the blackboard again when recreating them!
        self.active_offense = OffenseClass.new(self.blackboard)
        self.active_support = SupportClass.new(self.blackboard)
        
        -- Clean this specific team's memory safely
        self.blackboard:clear()
    end

    self.previous_offense_id = offense_id

    self.active_support:process(support_id, self.team_id)
    self.active_offense:process(offense_id, self.team_id, support_id)
end

return TeamBehavior