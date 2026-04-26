local move_skill = require("skills.move") -- Note: Do not include the .lua extension
local kick_skill = require("skills.kick")
local go_to_ball = require("skills.go_to_ball")
local kick_to_point = require("skills.kick_to_point")
local robotId = 0
local team = 0

function process()
   kick_to_point.process(robotId, team, {x = 0.0, y = 0.0})
end