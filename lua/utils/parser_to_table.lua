local parsermodule = {}

-- ==========================================
-- LUA BLUEPRINT / FACTORY
-- ==========================================
local function create_new_play()
    return {
        name = nil,         -- Expected: string
        applicable = nil,   -- Expected: string
        done = nil,         -- Expected: string
        orole = nil,        -- Expected: table { id = number, assign = string }
        roles = {}          -- Expected: table [role_id] = { {tactic = string, param = table}, ... }
    }
end

-- ==========================================
-- PARSER HELPERS
-- ==========================================
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

-- ==========================================
-- MAIN PARSER
-- ==========================================
function parsermodule.parse_play(ruta_archivo)
    local archivo = assert(io.open(ruta_archivo, "r"))
    
    -- Initialize using the Blueprint
    local play = create_new_play()
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
                -- Initialize the role directly as an array
                play.roles[rol_actual] = {}

            elseif rol_actual ~= nil then
                -- Insert tactics directly into the role's array
                table.insert(play.roles[rol_actual], {
                    tactic_name = comando, 
                    param = limpiar_parametros(parametros)
                })
            end
        end
    end
    
    archivo:close()
    return play
end

return parsermodule