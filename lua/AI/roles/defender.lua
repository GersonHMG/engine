local mark_tactic = require("tactics.non_active.mark") -- Update this path if you saved it elsewhere
local utils = require("utils.utils")

local Defender = {}
Defender.__index = Defender

--- Initializes the Defender role
--- @param team_blackboard table - The team's isolated shared memory
function Defender.new(team_blackboard)
    return setmetatable({
        blackboard = team_blackboard,
        -- Instantiate our marking tactic so it can maintain its state
        mark = mark_tactic.new() 
    }, Defender)
end

--- Executes the continuous man-to-man defensive role
--- @param robotId number
--- @param team number
function Defender:process(robotId, team)
    local robot = get_robot_state(robotId, team)
    local ball = get_ball_state()

    if not robot or not ball then
        return
    end

    draw_text(robot.x, robot.y + 0.2, "Defender (Marking)", {r=1.0, g=0.5, b=0.0})

    -- 1. Determine the enemy team ID (Assuming teams are 0 and 1)
    local enemy_team = (team == 0) and 1 or 0

    -- 2. Find which enemy robot is closest to the ball (The Ball Carrier)
    local enemy_0 = get_robot_state(0, enemy_team)
    local enemy_1 = get_robot_state(1, enemy_team)

    local target_enemy_id = nil
    local min_distance = math.huge

    if enemy_0 then
        local dist_0 = utils.getDistance(enemy_0, ball)
        if dist_0 < min_distance then
            min_distance = dist_0
            target_enemy_id = 0
        end
    end

    if enemy_1 then
        local dist_1 = utils.getDistance(enemy_1, ball)
        if dist_1 < min_distance then
            min_distance = dist_1
            target_enemy_id = 1
        end
    end
    
    -- 3. If we successfully identified an enemy, execute the mark tactic
    if target_enemy_id ~= nil then
        self.mark:process(robotId, team, target_enemy_id, enemy_team)
    end
end

return {
    new = function(bb) return Defender.new(bb) end
}