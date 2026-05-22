local play_runner = require("lua.utils.run_play")

local PlayManager = {}
PlayManager.__index = PlayManager

-- Constructor
function PlayManager.new(play_path)
    local self = setmetatable({}, PlayManager)
    
    self.play_path = play_path
    
    -- Load the play initially upon instantiation
    play_runner.load_play(self.play_path)
    
    return self
end

-- Class Method: Process
function PlayManager:process()
    -- Run the play. If it returns true, it means all tactics are "none" (finished)
    if play_runner.process() == true then
        -- Reload the play
        print("Play finished. Reloading: " .. self.play_path)
        play_runner.load_play(self.play_path)
        
        return true -- Optional: return true to notify the main loop it reset
    end
    
    return false
end

return PlayManager