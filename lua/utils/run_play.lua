local parser = require("utils.parser_to_table")
local assign_roles = require("utils.utils").assign_roles

local TEAM_ID = 0

local Runner = {}

local tacticas_disp = {
    pass = require("tactics.active.pass"),
    position_for_pass = require("tactics.non_active.position_for_pass"),
    receive_pass = require("tactics.active.receive_pass"),
    none = {
        process = function() return false end
    }
}

local roles_to_id = {}
local play_actual = {}
local previous_state = {}

-- debugging helper to print params in a readable way
local function params_to_string(params)
    if type(params) == "table" then
        local result = {}
        for _, v in pairs(params) do
            table.insert(result, params_to_string(v))
        end
        for k, v in pairs(params) do
            if type(k) ~= "number" then
                table.insert(result, tostring(k) .. "=" .. params_to_string(v))
            end
        end
        return "{" .. table.concat(result, ", ") .. "}"
    elseif params == nil then
        return ""
    end
    return tostring(params)
end
------------------------------------------------------------------

-- Cargar la jugada desde un archivo y comenzar la ejecución
function Runner.load_play(ruta_archivo)
    -- from .play to table
    play_actual = parser.parse_play(ruta_archivo)
    -- Map role to id
    roles_to_id = assign_roles(TEAM_ID)
end

function next_tactic(role_id)
    local tactics = play_actual.roles[role_id]
    if tactics[1].tactic_name == "none" then
        return
    end

    -- Remove the completed tactic
    table.remove(tactics, 1)

    -- Return the next tactic or nil if no more tactics
    return tactics[1]
end

-- Helper function to find "role_x" strings and replace them with the robot_id
local function resolve_role_params(params, roles_to_id)
    if type(params) == "table" then
        local resolved_table = {}
        for k, v in pairs(params) do
            if type(v) == "table" then
                -- Recursively resolve nested tables
                resolved_table[k] = resolve_role_params(v, roles_to_id)
            elseif type(v) == "string" then
                -- Check if the string matches "role_x" (where x is a number)
                local extracted_role_id = v:match("^role_(%d+)$")
                if extracted_role_id then
                    local role_num = tonumber(extracted_role_id)
                    -- Replace with robot_id if found, otherwise leave the original string
                    resolved_table[k] = roles_to_id[role_num] or v
                else
                    resolved_table[k] = v
                end
            else
                -- Pass through numbers, booleans, etc., unchanged
                resolved_table[k] = v
            end
        end
        return resolved_table
    elseif type(params) == "string" then
        -- Handle the case where params is just a single string value
        local extracted_role_id = params:match("^role_(%d+)$")
        if extracted_role_id then
            local role_num = tonumber(extracted_role_id)
            return roles_to_id[role_num] or params
        end
        return params
    end
    
    -- Pass through if it's just a single number
    return params
end

-- run play
function Runner.process()
    -- If there's no play, technically all tactics are "none" (it's finished/empty)
    if not play_actual or not play_actual.roles then return true end

    local all_tactics_none = true

    for role_id, tactics in pairs(play_actual.roles) do
        local robot_id = roles_to_id[role_id]

        if not robot_id then
            print("No robot assigned for role " .. role_id)
            goto continue
        end

        -- Get the current tactic being played for this role based on its state
        local current_tactic = tactics[1]
        
        -- If a role runs out of tactics entirely, we treat it as done
        if not current_tactic then
            goto continue
        end

        -- If ANY role has an active tactic that isn't "none", the play isn't finished yet
        if current_tactic.tactic_name ~= "none" then
            all_tactics_none = false
        end

        local tactic = tacticas_disp[current_tactic.tactic_name]
        
        -- Saftey check in case a tactic name is misspelled in the play file
        if not tactic then
            print("Warning: Tactic '" .. tostring(current_tactic.tactic_name) .. "' not found.")
            goto continue
        end
        
        -- ---------------------------------------------------------
        -- CONVERT "role_x" TO ROBOT IDs IN PARAMS
        -- ---------------------------------------------------------
        local mapped_params = resolve_role_params(current_tactic.param, roles_to_id)
        
        -- DEBUG PRINTING
        local actual_tactic = current_tactic.tactic_name
        local text_debug = string.format("R%s: %s", tostring(robot_id), tostring(actual_tactic))

        if previous_state[robot_id] ~= actual_tactic then
            if actual_tactic ~= "none" then
                print(text_debug .. " " .. params_to_string(mapped_params))
            end
            previous_state[robot_id] = actual_tactic
        end
        
        local robot_pos = get_robot_state(robot_id, TEAM_ID)
        if robot_pos then
            local pos_x = robot_pos.x
            local pos_y = robot_pos.y + 0.08

            draw_text(pos_x, pos_y, text_debug, {r=1.0, g=1.0, b=1.0})
        end
        ------------    
        
        local is_tactic_done = false
        if type(mapped_params) == "table" and #mapped_params > 0 then
            is_tactic_done = tactic.process(robot_id, TEAM_ID, table.unpack(mapped_params))
        else
            is_tactic_done = tactic.process(robot_id, TEAM_ID, mapped_params)
        end

        if is_tactic_done then
            next_tactic(role_id)
        end

        ::continue::
    end

    return all_tactics_none
end

return Runner