local plays = require("AI.calculator.play_assigner")  -- adjust path to where this file lives

function process()
   local roles, ordered = plays.assign_plays(0)
    for _, entry in ipairs(ordered) do
        local r = get_robot_state(entry.id, 0)
        draw_text(r.x, r.y +  0.2, string.format("%d : %s", entry.id, entry.role))
    end


end
