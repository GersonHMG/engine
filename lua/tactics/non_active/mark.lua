local utils = require("utils.utils")

local Mark = { State = { playing = "PLAYING", failed = "FAILED", done = "DONE" } }
Mark.__index = Mark

function Mark.new()
    return setmetatable({ state = Mark.State.playing }, Mark)
end

--- Executes the marking tactic to automatically block the enemy with the ball
--- @param robotId number - Your marking robot
--- @param team number - Your team ID (0 or 1)
function Mark:process(robotId, team)
    if self.state ~= Mark.State.playing then
        return self.state == Mark.State.done
    end

    local robot = get_robot_state(robotId, team)
    local ball = get_ball_state()

    if not robot or not ball then
        self.state = Mark.State.failed
        return false
    end

    -- 1. Deduce the enemy team ID (Assuming 0 is Blue, 1 is Yellow)
    local enemyTeam = (team == 0) and 1 or 0

    -- 2. Find the enemy robot closest to the ball
    local enemy_with_ball = nil
    local min_dist = math.huge

    -- Loop through both enemy robots (IDs 0 and 1)
    for i = 0, 1 do
        local enemy_bot = get_robot_state(i, enemyTeam)
        if enemy_bot then
            local dist = utils.getDistance(enemy_bot, ball)
            if dist < min_dist then
                min_dist = dist
                enemy_with_ball = enemy_bot
            end
        end
    end

    -- If no enemies are found on the field, fail the state
    if not enemy_with_ball then
        self.state = Mark.State.failed
        return false
    end

    -- 3. Execute the marking math against the found enemy
    local MARK_DISTANCE = 0.5 
    local dist_enemy_to_ball = min_dist -- We already calculated this!

    local target_x = enemy_with_ball.x
    local target_y = enemy_with_ball.y

    if dist_enemy_to_ball > 0.001 then
        local dir_x = (ball.x - enemy_with_ball.x) / dist_enemy_to_ball
        local dir_y = (ball.y - enemy_with_ball.y) / dist_enemy_to_ball

        local actual_distance = math.min(MARK_DISTANCE, dist_enemy_to_ball - 0.1)
        if actual_distance < 0 then actual_distance = 0 end

        target_x = enemy_with_ball.x + (dir_x * actual_distance)
        target_y = enemy_with_ball.y + (dir_y * actual_distance)
    end

    -- Draw an orange point for debugging
    draw_point(target_x, target_y, true, {r=1.0, g=0.5, b=0.0})
    
    move_to(robotId, team, {x = target_x, y = target_y})
    face_to(robotId, team, {x = ball.x, y = ball.y})

    return false
end

function Mark:get_state()
    return self.state
end

function Mark:reset()
    self.state = Mark.State.playing
end

local def = Mark.new()

return {
    State = Mark.State,
    new = Mark.new,
    -- Note: Removed enemyId and enemyTeam from the export wrapper
    process = function(r, t) return def:process(r, t) end,
    get_state = function() return def:get_state() end,
    reset = function() def:reset() end
}