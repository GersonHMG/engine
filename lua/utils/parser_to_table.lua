local parsermodule = {}


local function limpiar_parametros(texto)
    local lista_limpia = {}
    if not texto or texto == "" then
        return lista_limpia
    end

    for token in texto:gmatch("[%w%-%.%_]+") do
        local valor = tonumber(token) or token  -- Intentamos convertir a número, si no se puede, queda como string
        table.insert(lista_limpia, valor)
    end
    return lista_limpia
end

function parsermodule.parse_play(ruta_archivo)
    local archivo = assert(io.open(ruta_archivo, "r"))
    local play = {roles = {}}
    local rol_actual = nil

for linea in archivo:lines() do
    if linea ~= "" then
        local comando, parametros = linea:match("^%s*(%S+)%s*(.-)$")

        if comando == "PLAY" then
            play.name = parametros

        elseif comando == "APPLICABLE" then
            play.applicable = parametros

        elseif comando == "DONE" then
            play.done = parametros

        elseif comando == "OROLE" then
            local id_rol, condicion = parametros:match("^(%d+)%s*(.+)$")
            play.orole = {
                id = tonumber(id_rol),
                assign = condicion
            }

        elseif comando == "ROLE" then
            rol_actual = tonumber(parametros)
            play.roles[rol_actual] = {tactics = {}}

        elseif rol_actual ~= nil then
            table.insert(play.roles[rol_actual].tactics, 
            {action = comando, param = limpiar_parametros(parametros)})
        end

    end
end
archivo:close()
end
return parsermodule