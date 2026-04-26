local move_skill = require("skills.move") -- Note: Do not include the .lua extension
local kick_skill = require("skills.kick")
local go_to_ball = require("skills.go_to_ball")
local robotId = 0
local team = 0

local path = {
    {x = 1.0, y = 0.0},
    {x = 0.0, y = 1.0},
    {x = 2.0, y = 0.0}
}


function process()
    move_along_path(0, 0, path)
end