local go_to_ball = {}


function go_to_ball.is_on_point(robot_id, team, target, tolerance)
    local r_state = get_robot_state(robot_id, team)
    return math.sqrt((r_state.x - target.x)^2 + (r_state.y - target.y)^2) <= tolerance
end


function go_to_ball.process(robotId, team)
    -- Move to ball
    local ball_pos = get_ball_state()
    local tolerance = 0.2
    if go_to_ball.is_on_point(robotId, team, ball_pos, tolerance) then
        return true
    end
    
    move_to(robotId, team, ball_pos)
    return false
end

return go_to_ball