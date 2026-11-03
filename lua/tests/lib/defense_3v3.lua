-- defense_3v3: the yellow defense of the 3v3 headless test (lua/tests/attack.lua).
--   yellow 1 keeps the goal blue attacks (x = +4.5),
--   yellow 0 marks the blue field robot farther from the ball (cuts passes to it),
--   yellow 2 guards the ball: it stands GUARD_DISTANCE in front of the ball, on the line
--            ball -> goal, so a straight shot or a forward kick has to go around it.
-- Blue's field robots are the two farther from blue's own goal (the third is its keeper).
local mark = require("skills.mark")
local goalkeeper = require("skills.goalkeeper")
local profile = require("utils.robot_profile")

local M = {}

M.YELLOW = 1
M.MARKER_ID = 0
M.KEEPER_ID = 1
M.GUARD_ID = 2
M.GOAL = { x = 4.5, y = 0.0 }       -- the goal yellow defends (blue attacks it)
M.BLUE_GOAL = { x = -4.5, y = 0.0 }
M.BLUE_IDS = { 0, 1, 2 }
M.GUARD_DISTANCE = profile.opponent.guard_distance or 0.35  -- m, ball to the guard spot
M.AREA_X, M.AREA_HALF_Y = 3.5, 1.0  -- yellow's defense area (x >= AREA_X, |y| <= AREA_HALF_Y)
M.AREA_MARGIN = 0.12                -- m, the guard stays this far outside it

local function dist(a, b)
    return math.sqrt((a.x - b.x) ^ 2 + (a.y - b.y) ^ 2)
end

--- Blue's active robots except the one nearest blue's own goal (its keeper, with 3+).
local function blue_field()
    local robots = {}
    for _, id in ipairs(M.BLUE_IDS) do
        local r = get_robot_state(id, 0)
        if r and r.active then robots[#robots + 1] = r end
    end
    if #robots >= 3 then
        table.sort(robots, function(a, b) return dist(a, M.BLUE_GOAL) < dist(b, M.BLUE_GOAL) end)
        table.remove(robots, 1)
    end
    return robots
end

--- The blue field robot farther from the ball.
--- @return integer|nil
function M.free_attacker()
    local ball = get_ball_state()
    local best, best_d = nil, -1
    for _, r in ipairs(blue_field()) do
        local d = dist(r, ball)
        if d > best_d then best, best_d = r.id, d end
    end
    return best
end

--- The guard spot: GUARD_DISTANCE from the ball towards the goal, kept out of the
--- defense area.
function M.guard_spot(ball)
    local length = math.max(dist(ball, M.GOAL), 1e-3)
    local spot = {
        x = ball.x + (M.GOAL.x - ball.x) * M.GUARD_DISTANCE / length,
        y = ball.y + (M.GOAL.y - ball.y) * M.GUARD_DISTANCE / length,
    }
    if spot.x > M.AREA_X - M.AREA_MARGIN and math.abs(spot.y) < M.AREA_HALF_Y + M.AREA_MARGIN then
        spot.x = M.AREA_X - M.AREA_MARGIN
    end
    return spot
end

--- Runs one tick of the yellow defense.
function M.process()
    local target = M.free_attacker()
    if target then
        mark.process(M.MARKER_ID, M.YELLOW, target)
    end
    goalkeeper.process(M.KEEPER_ID, M.YELLOW, M.GOAL)
    local ball = get_ball_state()
    local spot = M.guard_spot(ball)
    move_to(M.GUARD_ID, M.YELLOW, spot)
    face_to(M.GUARD_ID, M.YELLOW, { x = ball.x, y = ball.y })
end

return M
