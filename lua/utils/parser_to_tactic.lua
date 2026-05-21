local parser = require("utils.parser_to_table")

local Runner = {}
local tacticas_disp = {
    pass = require("tactics.active.pass"),
    position_for_pass = require("tactics.active.position_for_pass"),
    receive_pass = require("tactics.active.receive_pass"),
    none = {
        process = function() return false end
    }
}

local estado_rol = {}
local play_actual = nil

local function asignar_rol(id_rol, orole)
    --falta implementar logica.
end

-- Cargar la jugada desde un archivo y comenzar la ejecución
function Runner.cargar_jugada(ruta_archivo)
    play_actual = parser.parse_play(ruta_archivo)
    Runner.begin_play(play_actual)
end

-- Iniciar la ejecución de la jugada
function Runner.begin_play(playparseada)
    estado_rol = {}
    if not play_actual then return end
    for id_rol, data_rol in pairs(playparseada.roles) do
        estado_rol[id_rol] = 1
    end
end

function Runner.update(playparseada, teamId)
    if not play_actual then return end
    for id_rol, data_rol in pairs(playparseada.roles) do
        local robotId = asignar_rol(id_rol, playparseada.orole) -- Falta implementar asignar_rol
        if robotId then
            local paso_actual = estado_rol[id_rol]
            local tactica_actual = data_rol.tactics[paso_actual]
            if tactica_actual then
                local modulo_tactica = tacticas_disp[tactica_actual.action]
                if modulo_tactica then
                    local termino_tarea = modulo_tactica.process(robotId, teamId, tactica_actual.param)
                    if termino_tarea then
                        estado_rol[id_rol] = paso_actual + 1
                    end
                end
            end
        end
    end
end
return Runner