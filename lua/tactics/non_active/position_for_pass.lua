local position_for_pass = {}

--- Helper function to calculate distance between two points
local function get_distance(p1, p2)
    return math.sqrt((p2.x - p1.x)^2 + (p2.y - p1.y)^2)
end

--- Evaluates the region and returns the best point for a pass
--- @param robotId number
--- @param team number
--- @param region table {min_x, max_x, min_y, max_y}
function position_for_pass.process(robotId, team, shape, p1, p2)

    local region = {
        min_x = math.min(p1[1], p2[1]),
        max_x = math.max(p1[1], p2[1]),
        min_y = math.min(p1[2], p2[2]),
        max_y = math.max(p1[2], p2[2])
    }

    local ball_pos = get_ball_state()
    local robot_pos = get_robot_state(robotId, team)
    
    -- 1. Start the best point with the robot's current position
    local best_point = {x = robot_pos.x, y = robot_pos.y}
    local best_score = -math.huge
    
    local step_size = 0.05

    -- Define the optimal distance to stay away from the ball to avoid bounces
    -- (Adjust this value based on your coordinate units, e.g., 0.5 meters or 500 mm)
    local IDEAL_BALL_DIST = 0.8

    -- Iterate over the given region
    for x = region.min_x, region.max_x, step_size do
        for y = region.min_y, region.max_y, step_size do
            local test_point = {x = x, y = y}
            local point_score = 0
            
            -- 1. Penalize moving far from the robot's current position
            local dist_to_robot = get_distance(robot_pos, test_point)
            point_score = point_score - dist_to_robot
            
            -- 2. "Sweet Spot" Ball Distance Metric
            local dist_to_ball = get_distance(ball_pos, test_point)
            
            -- Optional: Add a hard rejection if it's dangerously close
            if dist_to_ball < 1.0 then
                point_score = point_score - 1000 -- Massive penalty to forbid this point
            else
                -- Penalize how far the point is from the IDEAL distance
                local ball_dist_error = math.abs(dist_to_ball - IDEAL_BALL_DIST)
                point_score = point_score - (ball_dist_error * 0.5)
            end
            
            -- Update best point if this one is better
            if point_score > best_score then
                best_score = point_score
                best_point = test_point
            end
        end
    end

    -- 3. Execute movement and ALWAYS face the ball
    draw_point(best_point.x, best_point.y, true, {r=0.0, g=1.0, b=0.0}) -- Green point for the best position
    
    move_to(robotId, team, best_point)
    face_to(robotId, team, {x = ball_pos.x, y = ball_pos.y} )

    local dist_to_obj = get_distance(robot_pos, best_point)
    local TOLERANCIA = 0.01

    if dist_to_obj < TOLERANCIA then
        return true
    else
        return false
    end
end

return position_for_pass