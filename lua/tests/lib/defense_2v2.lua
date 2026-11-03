-- defense_2v2: the yellow defense of lua/run_ai.lua, shared with the headless tests so
-- both play exactly the same opponent.
--   yellow 0 marks the blue robot farther from the ball (cuts passes to it),
--   yellow 1 keeps the goal blue attacks (x = +4.5).
local mark = require("skills.mark")
local goalkeeper = require("skills.goalkeeper")

local M = {}

M.YELLOW = 1
M.MARKER_ID = 0
M.KEEPER_ID = 1
M.GOAL = { x = 4.5, y = 0.0 }  -- the goal yellow defends (blue attacks it)
M.BLUE_IDS = { 0, 1 }

--- The blue robot farther from the ball: the free attacker a pass would go to.
--- @return integer|nil
function M.free_attacker()
    local ball = get_ball_state()
    local best, best_d = nil, -1
    for _, id in ipairs(M.BLUE_IDS) do
        local r = get_robot_state(id, 0)
        if r and r.active then
            local d = math.sqrt((r.x - ball.x) ^ 2 + (r.y - ball.y) ^ 2)
            if d > best_d then best, best_d = id, d end
        end
    end
    return best
end

--- Runs one tick of the yellow defense.
function M.process()
    local target = M.free_attacker()
    if target then
        mark.process(M.MARKER_ID, M.YELLOW, target)
    end
    goalkeeper.process(M.KEEPER_ID, M.YELLOW, M.GOAL)
end

return M
