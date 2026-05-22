-- If saving in a separate file, uncomment the line below to import your module:
local parsermodule = require("utils.parser_to_table")

-- Recursive function to format and print a Lua table
local function dump_table(obj, indent)
    indent = indent or 0
    local formatting = string.rep("  ", indent)

    if type(obj) == 'table' then
        local s = '{\n'
        for k, v in pairs(obj) do
            -- Format keys (numbers get brackets, strings get quotes)
            local key
            if type(k) == 'number' then
                key = "[" .. k .. "]"
            else
                key = tostring(k)
            end
            
            s = s .. formatting .. "  " .. key .. " = " .. dump_table(v, indent + 1) .. ",\n"
        end
        return s .. formatting .. '}'
    elseif type(obj) == 'string' then
        -- Wrap strings in quotes for clarity
        return string.format("%q", obj)
    else
        return tostring(obj)
    end
end

-- ==========================================
-- Execution Script
-- ==========================================

-- 1. Define the path to your play file
local file_path = "lua/plays/pass.play"

-- 2. Parse the play file
local play_table = parsermodule.parse_play(file_path)

-- 3. Print the resulting table
print("--- Parsed Play Output ---")
print(dump_table(play_table))