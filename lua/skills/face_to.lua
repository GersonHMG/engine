
local function is_facing_point(id, equipo, target, tolerancia)
    local robot_state = get_robot_state(id, equipo)
    
    -- math.atan2 (o atan dependiendo de Lua) saca el ángulo entre dos puntos
    local target_angle = math.atan(target.y - robot_state.y, target.x - robot_state.x)
    
    -- Vemos la diferencia absoluta entre donde mira el robot y donde queremos que mire
    local angle_diff = math.abs(robot_state.orientation - target_angle)
    
    -- Normalizamos para que no de vueltas locas (lo mantenemos entre -PI y PI)
    while angle_diff > math.pi do
        angle_diff = angle_diff - (2 * math.pi)
    end
    
    return angle_diff <= tolerancia
end