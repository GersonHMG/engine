local pass = {}

-- Importamos la lógica robusta de chute que ya te funciona
local kick = require("skills.kick_to_point") 

--- Ejecuta el pase hacia un compañero específico
--- @param robotId number - El robot que tiene la pelota (el que da el pase)
--- @param team number - Tu equipo
--- @param targetRobotId number - El ID de tu compañero (el que recibe)
function pass.process(robotId, team, targetRobotId)
    
    -- 1. Obtenemos el estado (posición y rotación) del compañero
    local mate_state = get_robot_state(targetRobotId, team)
    
    -- 2. Armamos la coordenada objetivo basada en dónde está el pana
    local target_point = {
        x = mate_state.x,   
        y = mate_state.y
    }
    
    -- 3. Invocamos tu función de chute hacia esa coordenada
    -- Esto retornará true si ya pateó, o false si se está acomodando
    return kick.process(robotId, team, target_point)
end

return pass