local intercept = {}

function get_distance(p1, p2)
    local dx = p2.x - p1.x
    local dy = p2.y - p1.y
    return math.sqrt(dx^2 + dy^2)
end

--- Executes the intercept tactic
--- @param robotId number
--- @param team number
function intercept.process(robotId, team)
    local ball_pos = get_ball_state()
    local robot_pos = get_robot_state(robotId, team)
    
    
    local bx = ball_pos.x
    local by = ball_pos.y
    local bvx = ball_pos.vel_x
    local bvy = ball_pos.vel_y
    local rx = robot_pos.x
    local ry = robot_pos.y
    
    local target_x, target_y
    
    -- Calculamos la velocidad al cuadrado para ver si la pelota se mueve
    local speed_sq = (bvx^2) + (bvy^2)

    -- Si la pelota se está moviendo a una velocidad decente (ajusta este umbral)
    if speed_sq > 0.05 then
        -- 1. Vector desde la pelota hacia el robot
        local wx = rx - bx
        local wy = ry - by
        
        -- 2. Producto punto (dot product) para proyectar el robot en la trayectoria de la pelota
        -- Esto nos da un escalar 'c' que indica qué tan lejos en la línea de la pelota está la intercepción
        local c = ((wx * bvx) + (wy * bvy)) / speed_sq
        
        if c > 0 then
            -- La pelota va en dirección hacia la zona del robot (c > 0)
            -- Calculamos el punto ortogonal en la trayectoria
            target_x = bx + (c * bvx)
            target_y = by + (c * bvy)
        else
            -- El pase fue pa atrás o la pelota se está alejando.
            -- Aquí el robot se queda donde está o puedes mandarlo a una posición base.
            target_x = rx
            target_y = ry
        end
    else
        local OFFSET = 0.15
        local dx = rx - bx
        local dy = ry - by
        local dist = math.sqrt(dx^2 + dy^2)
        
        if dist > 0.001 then
            target_x = bx + (dx / dist) * OFFSET
            target_y = by + (dy / dist) * OFFSET
        else
            target_x = bx
            target_y = by
        end
    end

    draw_point(target_x, target_y, true, {r=0.0, g=1.0, b=0.0}) -- Puntito verde pal target

    move_to(robotId, team, {x = target_x, y = target_y})
    face_to(robotId, team, {x = bx, y = by})
end

return intercept