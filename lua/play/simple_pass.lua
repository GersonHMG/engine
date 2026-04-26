local simple_pass_play = {}

-- State variable to track Role 2's current action
local role2_state = "POSITIONING"

-- Define the region where Role 2 should wait for the pass
local PASS_REGION = { min_x = 1.0, max_x = 4.0, min_y = -3.0, max_y = 3.0 }

local pass_to = require("tactics.active.pass")

--- Phase 2: Executes the behavior for both roles
--- @param role1_id number
--- @param role2_id number
--- @param team string|number
--- @return string: Status of the play ("RUNNING" or "COMPLETED")
function simple_pass_play.execute(role1_id, role2_id, team)
    local ball_pos = get_ball_state()

    -- ROLE 1
    pass_to(role1_id, team, role2_id)


    -- ==========================================
    -- ROLE 2: Position, then Receive
    -- ==========================================
    -- Calculate ball speed to determine if the pass was fired
    local vx = ball_pos.vx or 0
    local vy = ball_pos.vy or 0
    local speed_sq = (vx * vx) + (vy * vy)

    -- State Transition: If the ball is moving fast, it was kicked! Switch to receiving.
    -- (0.5 is a threshold for speed squared, tweak based on your physics)
    if role2_state == "POSITIONING" and speed_sq > 0.5 then
        role2_state = "RECEIVING"
    end

    -- State Execution
    if role2_state == "POSITIONING" then
        position_for_pass.process(role2_id, team, PASS_REGION)
    
    elseif role2_state == "RECEIVING" then
        receive_pass.process(role2_id, team)
        
        -- Check if the play is finished (Role 2 successfully caught it)
        if has_the_ball(role2_id, team) then
            return "COMPLETED"
        end
    end

    return "RUNNING"
end

return simple_pass_play