-- run_trajectory_tests.lua — Trajectory Tracking Integration Tests Suite
-- Implements Phase 4 of specs/trajectory_tracking_test.md

local generator = require("utils.trajectory_generator")
local TrajectoryTester = require("utils.trajectory_tester")

-- Test states
local STATE_INIT = 0
local STATE_RUNNING_LINEAR = 1
local STATE_RUNNING_CIRCULAR = 2
local STATE_RUNNING_S_CURVE = 3
local STATE_RUNNING_SQUARE = 4
local STATE_COMPLETED = 5

local current_state = STATE_INIT
local active_tester = nil
local results = {}

-- Config threshold defaults
local config = {
    robot_id = 0,
    team = 0,
    thresholds = {
        rmse_pos = 0.06,      -- metros
        rmse_theta = 0.15,    -- radianes
        max_err_pos = 0.12,   -- metros
        max_err_theta = 0.3,  -- radianes
    }
}

-- Main loop process function called by engine
function process()
    if current_state == STATE_INIT then
        print("[TEST SUITE] Starting Trajectory Tracking Tests...")
        results = {}
        
        -- Start Linear Test
        active_tester = TrajectoryTester.new("trajectory_test_linear", config)
        local start_p = { x = -1.5, y = 0.0, theta = 0.0 }
        local end_p = { x = 1.5, y = 0.0, theta = 0.0 }
        local traj = generator.linear(start_p, end_p, 4.0)
        active_tester:start(traj)
        
        current_state = STATE_RUNNING_LINEAR
        print("[TEST SUITE] Running Test 1/4: Linear Trajectory...")
        
    elseif current_state == STATE_RUNNING_LINEAR then
        local finished = active_tester:update()
        if finished then
            table.insert(results, active_tester:evaluate())
            
            -- Transition to Circular Test
            active_tester = TrajectoryTester.new("trajectory_test_circular", config)
            local center = { x = 0.0, y = 0.0 }
            local radius = 1.0
            local start_angle = 0.0
            local traj = generator.circular(center, radius, start_angle, 6.0)
            active_tester:start(traj)
            
            current_state = STATE_RUNNING_CIRCULAR
            print("[TEST SUITE] Running Test 2/4: Circular Trajectory...")
        end
        
    elseif current_state == STATE_RUNNING_CIRCULAR then
        local finished = active_tester:update()
        if finished then
            table.insert(results, active_tester:evaluate())
            
            -- Transition to S-Curve Test
            active_tester = TrajectoryTester.new("trajectory_test_s_curve", config)
            local start_p = { x = -1.5, y = -1.0, theta = 0.0 }
            local end_p = { x = 1.5, y = 1.0, theta = 0.0 }
            local traj = generator.s_curve(start_p, end_p, 0.5, 1.0, 6.0) -- Amplitude 0.5m, Freq 1.0, 6s duration
            active_tester:start(traj)
            
            current_state = STATE_RUNNING_S_CURVE
            print("[TEST SUITE] Running Test 3/4: S-Curve Trajectory...")
        end
        
    elseif current_state == STATE_RUNNING_S_CURVE then
        local finished = active_tester:update()
        if finished then
            table.insert(results, active_tester:evaluate())
            
            -- Transition to Square Test
            active_tester = TrajectoryTester.new("trajectory_test_square", config)
            local corners = {
                { x = -1.0, y = -1.0, theta = 0.0 },
                { x = -1.0, y = 1.0,  theta = math.pi / 2.0 },
                { x = 1.0,  y = 1.0,  theta = math.pi },
                { x = 1.0,  y = -1.0, theta = -math.pi / 2.0 }
            }
            local traj = generator.square(corners, 8.0)
            active_tester:start(traj)
            
            current_state = STATE_RUNNING_SQUARE
            print("[TEST SUITE] Running Test 4/4: Square Trajectory...")
        end
        
    elseif current_state == STATE_RUNNING_SQUARE then
        local finished = active_tester:update()
        if finished then
            table.insert(results, active_tester:evaluate())
            
            current_state = STATE_COMPLETED
            print("[TEST SUITE] All tests completed! Printing summary:")
            
            -- Calculate and print overall results
            local suite_passed = true
            print("==================================================")
            print("                SUITE TEST SUMMARY                ")
            print("==================================================")
            for _, res in ipairs(results) do
                print(string.format(" - %-25s: %s", res.test_name, res.verdict))
                if res.verdict ~= "PASS" then
                    suite_passed = false
                end
            end
            print("==================================================")
            print(string.format(" OVERALL RESULT: %s", suite_passed and "PASS" or "FAIL"))
            print("==================================================")
            
            -- Send velocity zero to stop the robot
            send_velocity(config.robot_id, config.team, 0.0, 0.0, 0.0)
        end
        
    elseif current_state == STATE_COMPLETED then
        -- Keep drawing a final overlay on screen
        draw_text(-2.0, 2.0, "TRAJECTORY TESTS SUITE COMPLETED", {0.0, 1.0, 0.0})
        
        local suite_passed = true
        for _, res in ipairs(results) do
            if res.verdict ~= "PASS" then suite_passed = false end
        end
        
        local color = suite_passed and {0.0, 1.0, 0.0} or {1.0, 0.0, 0.0}
        draw_text(-2.0, 1.8, string.format("OVERALL RESULT: %s", suite_passed and "PASS" or "FAIL"), color)
        
        local y_offset = 1.5
        for i, res in ipairs(results) do
            local test_color = res.verdict == "PASS" and {0.0, 1.0, 0.0} or {1.0, 0.0, 0.0}
            draw_text(-2.0, y_offset, string.format("%d. %s: %s (RMSE pos: %.4fm)", i, res.test_name, res.verdict, res.metrics.rmse_position), test_color)
            y_offset = y_offset - 0.2
        end
    end
end
