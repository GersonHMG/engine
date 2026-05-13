local pase_estrategia = {}

-- 1. IMPORTAMOS las habilidades y tácticas que necesita esta estrategia
local pass = require("tactics.active.pass")
local recivepass = require("tactics.active.intercept")
local position_for_pass = require("tactics.non_active.position_for_pass")

-- Variable de estado (como es local, mantendrá su valor ciclo a ciclo)
local estado_jugada = "PREPARANDO_PASE"

local function get_distance(pos1, pos2)
    return math.sqrt((pos1.x - pos2.x)^2 + (pos1.y - pos2.y)^2)
end

-- 2. AMARRAMOS la función a la tabla y le pasamos los PARAMETROS
function pase_estrategia.process(robotId, team, targetRobotId)
    
    draw_text(0.5, 0.5, estado_jugada, {r=1.0, g=1.0, b=0.0})
   if estado_jugada == "PREPARANDO_PASE" then
      position_for_pass.process(targetRobotId, team, {min_x = 0.0, max_x = 3.0, min_y = 0.0, max_y = 2.0})
      
      local pase_completo = pass.process(robotId, team, targetRobotId)

      if pase_completo then
         estado_jugada = "BOLA_SALIENDO"
      end
      
      
   elseif estado_jugada == "BOLA_SALIENDO" then
      position_for_pass.process(targetRobotId, team, {min_x = 0.0, max_x = 3.0, min_y = 0.0, max_y = 2.0})

      local ball_pos = get_ball_state()
      local robot_pos = get_robot_state(robotId, team)
      local dist_pelota_pase = get_distance(ball_pos, robot_pos)

      if dist_pelota_pase > 0.15 then
         estado_jugada = "RECIBIENDO_PASE"
      end

   elseif estado_jugada == "RECIBIENDO_PASE" then
      recivepass.process(targetRobotId, team, robotId)
   end
end

return pase_estrategia