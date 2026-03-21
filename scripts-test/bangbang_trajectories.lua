local waypoints = {
    {x = 0.5, y = 0.2},
    {x = 1.0, y = 0.8},
    {x = 1.5, y = 0.3}
}

-- Add variables to track our progress
local current_index = 1
local step = 1 -- Use 1 to move forward through the list, -1 to move backward
local tolerance = 0.05 -- How close the robot needs to be to "reach" the point

function process()
    -- 1. Draw all the waypoints
    for index, p in ipairs(waypoints) do
        draw_point(p.x, p.y)
    end

    -- 2. Get the target point and the robot's current position
    local target = waypoints[current_index]
    -- Note: Ensure get_robot_state returns x, y (and possibly heading, but we only need x, y here)
    local rx, ry = get_robot_state(0, 0).x, get_robot_state(0, 0).y -- Using ID 0, Team 0

    -- 3. Calculate the distance between the robot and the target point
    local dx = target.x - rx
    local dy = target.y - ry
    local distance = math.sqrt((dx * dx) + (dy * dy))

    -- 4. If we are close enough to the target, advance to the next point
    if distance <= tolerance then
        -- Move to the next index based on our current direction
        current_index = current_index + step
        
        -- Check if we've gone past the last waypoint
        if current_index > #waypoints then
            step = -1                     -- Reverse direction to backward
            current_index = #waypoints - 1 -- Set target to the second-to-last point
            
        -- Check if we've gone past the first waypoint
        elseif current_index < 1 then
            step = 1                      -- Reverse direction to forward
            current_index = 2             -- Set target to the second point
        end
        
        -- Update to the new target
        target = waypoints[current_index]
    end

    -- 5. Send the trajectory command for ONLY the current target
    bangbang_trajectory(
        0,          -- robot id
        0,          -- team (0 blue, 1 yellow)
        5.0,        -- v_max
        2.5,        -- a_max
        { target }  -- sending just the active point
    )
end