local kick = {}

function kick.process(robotId, team)
    kickx(robotId, team) -- Ejecuta la patada plana inmediatamente
    return true          -- Le dice al sistema que la tarea ya terminó
end

return kick