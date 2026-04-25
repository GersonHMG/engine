local move_skill = require("skills.move") -- Note: Do not include the .lua extension
local kick_skill = require("skills.kick")

local robotId = 0
local team = 0

function process()
    kick_skill.process(robotId, team)
end