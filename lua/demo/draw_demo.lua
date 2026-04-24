-- Initial state
local robotId = 0
local team = 0
local current_point_index = 1

-- 1. Define the 4 corners of the square (Counter-Clockwise order)
local path = {
    {x = -2.0, y = -2.0}, -- 1: Bottom-Left (Start)
    {x = -2.0, y =  2.0}, -- 2: Top-Left (Moving UP first)
    {x =  2.0, y =  2.0}, -- 3: Top-Right
    {x =  2.0, y = -2.0}  -- 4: Bottom-Right
}

-- Helper to format the path for draw_line and close the square visually
local function get_line_coordinates()
    local coords = {}
    for i, p in ipairs(path) do
        table.insert(coords, {p.x, p.y})
    end
    -- Add the first point at the very end to visually close the square line
    table.insert(coords, {path[1].x, path[1].y})
    return coords
end

-- PLACEHOLDER: Remember to replace this with actual distance checking logic!
local function has_reached_point(id, t, target)
    local pos = get_robot_state(id, t)
    local dist = math.sqrt((pos.x - target.x)^2 + (pos.y - target.y)^2)
    return dist < 0.02
end

function process()
    -- 2. Visuals: Highlight active robot and draw the square
    highlight_robot(robotId, team)
    
    local line_coords = get_line_coordinates()
    draw_line(line_coords)

    -- Draw individual target points (the 4 corners)
    for i, p in ipairs(path) do
        draw_point(p.x, p.y)
    end

    -- 3. Movement and Handoff Logic
    local target_point = path[current_point_index]
    
    -- Issue the move command to the CURRENT active robot
    move_to(robotId, team, {x = target_point.x, y = target_point.y})
    
    -- Check if the active robot has arrived at the current corner
    if has_reached_point(robotId, team, target_point) then
        
        -- Target the next point in the counter-clockwise path
        current_point_index = current_point_index + 1
        
        -- Infinite Loop: if we pass the last point, wrap back to point 1
        if current_point_index > #path then
            current_point_index = 1
        end
        
        -- Handoff: Alternate to the next robot (toggle between team 0 and 1)
        if team == 0 then
            team = 1
        else
            team = 0
        end
    end
end