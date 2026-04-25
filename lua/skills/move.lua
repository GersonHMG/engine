local move = {}

function move.is_on_point(robot_id, team, target, tolerance)
    local r_state = get_robot_state(robot_id, team)
    return math.sqrt((r_state.x - target.x)^2 + (r_state.y - target.y)^2) <= tolerance
end

-- Main process loop
function move.process(robotId, team, target)
    local tolerance = 0.1
    if move.is_on_point(robotId, team, target, tolerance) then
        return true
    end

    move_to(robotId, team, target)
    return false
end

return move