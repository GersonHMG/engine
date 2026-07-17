local utils = require("utils.utils")
local receive_pass = require("tactics.active.receive_pass")

local ENABLE_ROLE_LOGGING = false

local Support = {}
Support.__index = Support

function Support.new(team_blackboard)
    local self = setmetatable({}, Support)
    -- Save the specific blackboard to this instance
    self.blackboard = team_blackboard
    self.logger = Logger.new("system_log", "RobotRole", false)
    return self
end

--- Executes the continuous support role to maintain an open passing lane
--- @param robotId number
--- @param team number
function Support:process(robotId, team)
    local ball = get_ball_state()
    local robot = get_robot_state(robotId, team)
    
    draw_text(robot.x, robot.y + 0.2, "Support", {r=0.0, g=1.0, b=0.0})

    if ENABLE_ROLE_LOGGING and self.logger then
        self.logger:log_csv({
            RobotID = robotId,
            Team = team,
            RobotRole = "support"
        })
    end
    
    if not ball or not robot then
        return
    end

    local dist_to_ball = utils.getDistance(robot, ball)
    local RECEIVE_THRESHOLD = 0.5

    if dist_to_ball < RECEIVE_THRESHOLD then
        receive_pass.process(robotId, team)
    else
        local OFFSET_X = 1.0
        local SPREAD_Y = 1.5

        local target_x = ball.x + OFFSET_X
        if target_x > 4.0 then target_x = 4.0 end

        local target_y
        if ball.y > 0 then
            target_y = ball.y - SPREAD_Y
        else
            target_y = ball.y + SPREAD_Y
        end

        -- PUBLISH TO THE BLACKBOARD:
        -- We save the target coordinate so the Offense role can read it later.
        self.blackboard:set("best_pass_point", { x = target_x, y = target_y })
        
        draw_point(target_x, target_y, true, {r=0.0, g=0.0, b=1.0})
        move_to(robotId, team, {x = target_x, y = target_y})
        face_to(robotId, team, {x = ball.x, y = ball.y})
    end
end

local def = Support.new()

return {
    new = Support.new,
    process = function(r, t) def:process(r, t) end
}