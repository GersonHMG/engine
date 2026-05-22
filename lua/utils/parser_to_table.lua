local parsermodule = {}


local function limpiar_parametros(texto)
    local lista_limpia = {}
    if not texto or texto == "" then
        return lista_limpia
    end

    local stack = {{}}

    local texto_espaciado = texto:gsub("{", " { "):gsub("}", " } ")

    for token in texto_espaciado:gmatch("%S+") do
        if token == "{" then
            table.insert(stack, {})
        elseif token == "}" then
            if #stack > 1 then
                local tabla_cerrada = table.remove(stack)
                table.insert(stack[#stack], tabla_cerrada)
            end
        else
            local valor = tonumber(token) or token
            table.insert(stack[#stack], valor)
        end
    end

    if #stack[1] == 1 and type(stack[1][1]) == "table" then
        return stack[1][1]
    end
    return stack[1]    
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
return play
end
return parsermodule