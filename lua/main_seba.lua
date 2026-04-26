local move_skill = require("skills.move") -- Note: Do not include the .lua extension
local kick_skill = require("skills.kick")
local go_to_ball = require("skills.go_to_ball")
local robotId = 0
local team = 0

function process()
    -- Ejecuta la acción de ir a la pelota.
    -- Retorna 'true' si el robot ya está en la posición de la pelota.
    local arrived_at_ball = go_to_ball.process(robotId, team)
    
    -- Si el robot ya llegó a la pelota, entonces ejecuta la acción de patear.
    if arrived_at_ball then
        kick_skill.process(robotId, team)
    end
end