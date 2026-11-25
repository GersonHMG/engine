-- ui_demo.lua — Script controls: parameters, buttons and field clicks.
--
-- Load it, open "Script controls" in the sidebar, set the parameters and
-- press Run. Ctrl+click the field to send the robot there (Ctrl+right click
-- to make it face that point instead); the buttons act while it runs or
-- while it is paused.

-- Parameters, declared at load time so they can be set before starting.
-- Read them with ui.get() inside process() to follow changes live.
ui.number("robot", 0)
ui.choice("team", "blue", { "blue", "yellow" })
ui.choice("mode", "move_to", { "move_to", "move_direct", "spin" })
ui.slider("spin speed", 3.0, 0.0, 10.0, 0.1)
ui.toggle("face ball", false)
ui.toggle("show path", true)

local target = nil -- where Ctrl+click sent the robot
local look_at = nil -- where Ctrl+right click made it face
local trail = {}

ui.button("stop", function()
    target = nil
    look_at = nil
end)

ui.button("center", function()
    target = { x = 0.0, y = 0.0 }
end)

ui.button("clear path", function()
    trail = {}
end)

ui.on_click(function(x, y, button)
    if button == "left" then
        target = { x = x, y = y }
    else
        look_at = { x = x, y = y }
    end
    print(("click %s at %.2f, %.2f"):format(button, x, y))
end)

function process()
    local id = math.floor(ui.get("robot"))
    local team = ui.get("team") == "blue" and 0 or 1
    local robot = get_robot_state(id, team)
    if not robot or not robot.active then
        return
    end

    local mode = ui.get("mode")
    if mode == "spin" then
        send_velocity(id, team, 0, 0, ui.get("spin speed"))
    elseif target then
        if mode == "move_to" then
            move_to(id, team, target)
        else
            move_direct(id, team, target)
        end
        draw_point(target.x, target.y, true, { 0.65, 0.55, 0.98 })
    else
        send_velocity(id, team, 0, 0, 0)
    end

    if ui.get("face ball") then
        face_to(id, team, get_ball_state())
    elseif look_at then
        face_to(id, team, look_at)
        draw_point(look_at.x, look_at.y, false, { 1.0, 0.7, 0.3 })
    end

    highlight_robot(id, team)
    if ui.get("show path") then
        trail[#trail + 1] = { robot.x, robot.y }
        if #trail > 600 then table.remove(trail, 1) end
        if #trail > 1 then draw_line(trail, false, { 0.4, 0.9, 0.5 }) end
    end
end
