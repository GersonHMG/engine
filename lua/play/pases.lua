local pases = {}

local state = 0
local pase = require("play.pass")


function pases.process(robotId, team, targetRobotId)
   if state == 0 then
      if pase.process(robotId, team, targetRobotId) then
         state = 1
      end
   elseif state == 1 then
      if pase.process(targetRobotId, team, robotId) then
         state = 0
      end
   end
end

return pases