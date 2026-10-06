-- Root entry point: run the AI framework experiment.
-- The engine loads this file and calls `process()` once per tick.
--
-- Every blue robot on the field runs the AI (attacking the goal at x = +4.5); roles are
-- assigned each tick (AI/calculator/role_assigner): attacker, one supporter, a goalkeeper
-- with 3+ robots, defenders for the rest. Add or remove blue robots freely. Yellow defends:
--   yellow 0 marks the blue robot farther from the ball (cuts passes to it),
--   yellow 1 keeps the goal.
-- Nothing is teleported: place the robots and the ball where you want (GUI or console),
-- then reload this file; they start from where they are.

local ai = require("AI.main")
local mark = require("skills.mark")
local goalkeeper = require("skills.goalkeeper")

local YELLOW = 1
local MARKER_ID = 0
local KEEPER_ID = 1
local YELLOW_GOAL = { x = 4.5, y = 0.0 }  -- the goal yellow defends (blue attacks it)
local BLUE_IDS = { 0, 1 }

-- The blue robot farther from the ball: the free attacker a pass would go to.
local function free_attacker()
    local ball = get_ball_state()
    local best, best_d = nil, -1
    for _, id in ipairs(BLUE_IDS) do
        local r = get_robot_state(id, 0)
        if r and r.active then
            local d = math.sqrt((r.x - ball.x) ^ 2 + (r.y - ball.y) ^ 2)
            if d > best_d then best, best_d = id, d end
        end
    end
    return best
end

function process()
    ai.process()

    local target = free_attacker()
    if target then
        mark.process(MARKER_ID, YELLOW, target)
    end
    goalkeeper.process(KEEPER_ID, YELLOW, YELLOW_GOAL)
end
