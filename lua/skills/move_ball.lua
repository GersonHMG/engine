-- Move the ball to a desired target

local move_ball = {}
local go_to_ball = require("skills.go_to_ball")
local utils = require("utils.utils")
local check_has_ball = utils.has_the_ball -- Cambiado el nombre de la importación

local function dist(x1, y1, x2, y2)
    return math.sqrt((x2 - x1)^2 + (y2 - y1)^2)
end

function move_ball.process(robotId, team, target_x, target_y)
    local tolerance = 0.1
    
    -- Variable local con un nombre distinto a la función
    local robot_has_ball = check_has_ball(robotId, team)
    
    -- 1. Si no tenemos la pelota, nuestra única misión es ir a buscarla
    if not robot_has_ball then
        go_to_ball.process(robotId, team)
        return false
    end

    -- 2. Si llegamos acá, significa que SÍ tenemos la pelota.
    -- Calculamos la distancia de la PELOTA al objetivo
    local ball_pos = get_ball_state()
    local distance_to_target = dist(ball_pos.x, ball_pos.y, target_x, target_y)
    
    -- 3. Decidimos qué hacer según la distancia
    if distance_to_target > tolerance then
        -- Aún no llegamos: Nos movemos y mantenemos el dribbler prendido
        move_direct(robotId, team, {x = target_x, y = target_y})
        dribbler(robotId, team, 5)
        return false
    else
        -- ¡Llegamos al objetivo! 
        -- (Opcional: puedes apagar el dribbler acá con dribbler(..., 0) si no quieres que siga girando)
        return true
    end
end

return move_ball