local utils = require("utils.utils")

local team = 0

function process()
    local robot_roles = utils.get_roles(team) -- Assuming teamId 0 for testing
    for roleId, robotId in pairs(robot_roles) do
        local robot = get_robot_state(robotId, team)
        draw_text(robot.x, robot.y + 0.1, roleId, {r=255,g=255,b=255})
    end
end