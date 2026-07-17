-- run_lateral_test.lua — Standalone Lateral Movement Test
local LateralTester = require("utils.lateral_tester")

local STATE_INIT = 0
local STATE_RUNNING = 1
local STATE_COMPLETED = 2

local current_state = STATE_INIT
local active_tester = nil
local result = nil

local config = {
    robot_id = 0,
    team = 0,
    thresholds = {
        rmse_y = 0.04,        -- metros
        max_drift_x = 0.05,   -- metros
        rmse_theta = 0.08,    -- radianes
        arrival_tolerance = 0.05,
    }
}

function process()
    if current_state == STATE_INIT then
        print("[LATERAL TEST] Starting lateral movement test...")
        active_tester = LateralTester.new("lateral_movement_test", config)
        local start_pos = { x = 0.0, y = 0.0, theta = 0.0 }
        active_tester:start(start_pos, 1.5, 4.0) -- start_pos, target_distance, duration
        current_state = STATE_RUNNING
        
    elseif current_state == STATE_RUNNING then
        local finished = active_tester:update()
        if finished then
            result = active_tester:evaluate()
            active_tester.logger:close()
            current_state = STATE_COMPLETED
            print("[LATERAL TEST] Test completed! Result: " .. result.verdict)
        end
        
    elseif current_state == STATE_COMPLETED then
        draw_text(-2.0, 2.0, "LATERAL TEST COMPLETED", {0.0, 1.0, 0.0})
        local color = result.verdict == "PASS" and {0.0, 1.0, 0.0} or {1.0, 0.0, 0.0}
        draw_text(-2.0, 1.8, "RESULT: " .. result.verdict, color)
        draw_text(-2.0, 1.6, string.format("RMSE Y (Lat): %.4fm (Limit: %.4fm)", result.metrics.rmse_lateral, result.thresholds.rmse_y), color)
        draw_text(-2.0, 1.4, string.format("Max Drift X:   %.4fm (Limit: %.4fm)", result.metrics.max_drift_x, result.thresholds.max_drift_x), color)
        draw_text(-2.0, 1.2, string.format("RMSE Theta:    %.4frad (Limit: %.4frad)", result.metrics.rmse_orientation, result.thresholds.rmse_theta), color)
    end
end
