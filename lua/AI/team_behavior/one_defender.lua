local DefenderClass = require("AI.roles.defender")
local BlackboardClass = require("AI.utils.blackboard")

local TeamBehavior = {}
TeamBehavior.__index = TeamBehavior

--- Initializes a 1-robot team dedicated entirely to defense
--- @param team_id number
--- @param robot_id number
function TeamBehavior.new(team_id, robot_id)
    local self = setmetatable({}, TeamBehavior)
    
    self.team_id = team_id
    self.robot_id = robot_id
    
    -- Create the private blackboard (just in case future tactics need it)
    self.blackboard = BlackboardClass.new()
    
    -- Instantiate ONLY the defender role
    self.active_defender = DefenderClass.new(self.blackboard)
    
    return self
end

function TeamBehavior:process()
    -- No role sorting needed! Just execute the defender logic constantly.
    self.active_defender:process(self.robot_id, self.team_id)
end

return TeamBehavior