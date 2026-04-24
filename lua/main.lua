local move_skill = require("skills.move") -- Note: Do not include the .lua extension

local robotId = 0
local team = 0

function process()
    move_skill.process(robotId, team, {x = 1.0, y = 1.0})
end