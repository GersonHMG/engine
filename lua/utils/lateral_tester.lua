local LateralTester = {}
LateralTester.__index = LateralTester

-- Helper to normalize angle to [-pi, pi]
local function normalize_angle(angle)
    while angle > math.pi do
        angle = angle - 2 * math.pi
    end
    while angle < -math.pi do
        angle = angle + 2 * math.pi
    end
    return angle
end

-- Constructor
function LateralTester.new(test_name, config)
    local self = setmetatable({}, LateralTester)
    self.test_name = test_name
    config = config or {}
    self.robot_id = config.robot_id or 0
    self.team = config.team or 0
    
    -- Merge config thresholds with defaults to avoid nil comparison runtime errors
    self.thresholds = {}
    local conf_thresholds = config.thresholds or {}
    self.thresholds.rmse_y = conf_thresholds.rmse_y or 0.04
    self.thresholds.max_drift_x = conf_thresholds.max_drift_x or 0.05
    self.thresholds.rmse_theta = conf_thresholds.rmse_theta or 0.08
    self.thresholds.arrival_tolerance = conf_thresholds.arrival_tolerance or 0.05

    self.finished = false
    self.samples = {}
    self.elapsed_time = 0
    self.logger = nil
    return self
end

-- Start the lateral movement test
function LateralTester:start(start_pos, target_distance, duration)
    self.start_pos = start_pos
    self.target_distance = target_distance
    self.duration = duration
    self.elapsed_time = 0
    self.finished = false
    self.samples = {}

    -- Teleport ball far away to avoid interference
    grsim.teleport_ball(5.0, 5.0)

    -- Teleport robot to start position
    grsim.teleport_robot(self.robot_id, self.team, start_pos.x, start_pos.y, start_pos.theta)

    -- Initialize logger with main = true and a dedicated log name
    -- This will create lateral_movement_test.csv and lateral_movement_test.json
    self.logger = Logger.new(self.test_name, {
        "ref_x", "ref_y", "ref_theta",
        "act_x", "act_y", "act_theta",
        "drift_x", "err_y", "err_theta",
        "cmd_vx", "cmd_vy", "cmd_w"
    }, true)
end

-- Update function executed at 60 Hz
function LateralTester:update()
    if self.finished then
        return true
    end

    local state = get_robot_state(self.robot_id, self.team)
    if not state or not state.active then
        return false
    end

    self.elapsed_time = self.elapsed_time + (1.0 / 60.0)

    -- Generate reference position based on linear progression of lateral distance
    local progress = math.min(self.elapsed_time / self.duration, 1.0)
    local ref_y_local = self.target_distance * progress

    -- Convert local reference position to global coordinates
    local ref_x = self.start_pos.x - ref_y_local * math.sin(self.start_pos.theta)
    local ref_y = self.start_pos.y + ref_y_local * math.cos(self.start_pos.theta)
    local ref_theta = self.start_pos.theta

    -- Calculate errors in the robot's local coordinate frame
    local dx = ref_x - state.x
    local dy = ref_y - state.y
    local drift_x = dx * math.cos(state.orientation) + dy * math.sin(state.orientation)
    local err_y = -dx * math.sin(state.orientation) + dy * math.cos(state.orientation)
    local err_theta = normalize_angle(ref_theta - state.orientation)

    -- Commands: vy moves the robot laterally; vx and w correct longitudinal drift and orientation
    local cmd_vy = self.target_distance / self.duration
    if self.elapsed_time >= self.duration then
        cmd_vy = 0.0
    end

    local cmd_vx = 1.5 * drift_x
    local cmd_w = 2.0 * err_theta

    send_velocity(self.robot_id, self.team, cmd_vx, cmd_vy, cmd_w)

    -- Log values to the csv file
    self.logger:log_csv({
        ref_x = ref_x,
        ref_y = ref_y,
        ref_theta = ref_theta,
        act_x = state.x,
        act_y = state.y,
        act_theta = state.orientation,
        drift_x = math.abs(drift_x),
        err_y = math.abs(err_y),
        err_theta = math.abs(err_theta),
        cmd_vx = cmd_vx,
        cmd_vy = cmd_vy,
        cmd_w = cmd_w
    })

    -- Store samples for statistics
    table.insert(self.samples, {
        drift_x = math.abs(drift_x),
        err_y = math.abs(err_y),
        err_theta = math.abs(err_theta)
    })

    -- Visual feedback in simulator
    highlight_robot(self.robot_id, self.team)
    draw_point(ref_x, ref_y, {0.0, 1.0, 0.0})
    draw_text(ref_x + 0.1, ref_y + 0.1, string.format("Lat t=%.2fs", self.elapsed_time), {1.0, 1.0, 1.0})

    -- Stop conditions (duration + 1 second stabilization time)
    if self.elapsed_time >= (self.duration + 1.0) then
        self.finished = true
        send_velocity(self.robot_id, self.team, 0.0, 0.0, 0.0)
        self:evaluate()
        return true
    end

    return false
end

-- Compute statistical metrics and save JSON report
function LateralTester:evaluate()
    local N = #self.samples
    if N == 0 then
        return { verdict = "FAIL", reason = "No samples gathered" }
    end

    local sum_sq_err_y = 0
    local sum_err_y = 0

    local sum_sq_err_theta = 0
    local sum_err_theta = 0

    local max_drift_x = 0

    for _, s in ipairs(self.samples) do
        sum_err_y = sum_err_y + s.err_y
        sum_sq_err_y = sum_sq_err_y + (s.err_y * s.err_y)

        sum_err_theta = sum_err_theta + s.err_theta
        sum_sq_err_theta = sum_sq_err_theta + (s.err_theta * s.err_theta)

        if s.drift_x > max_drift_x then
            max_drift_x = s.drift_x
        end
    end

    local mae_y = sum_err_y / N
    local rmse_y = math.sqrt(sum_sq_err_y / N)

    local mae_theta = sum_err_theta / N
    local rmse_theta = math.sqrt(sum_sq_err_theta / N)

    local pass_y_rmse = rmse_y <= self.thresholds.rmse_y
    local pass_drift = max_drift_x <= self.thresholds.max_drift_x
    local pass_theta_rmse = rmse_theta <= self.thresholds.rmse_theta

    local pass = pass_y_rmse and pass_drift and pass_theta_rmse

    local result = {
        test_name = self.test_name,
        verdict = pass and "PASS" or "FAIL",
        metrics = {
            samples = N,
            rmse_lateral = rmse_y,
            mae_lateral = mae_y,
            max_drift_x = max_drift_x,
            rmse_orientation = rmse_theta,
            mae_orientation = mae_theta,
            execution_time_seconds = self.elapsed_time
        },
        thresholds = self.thresholds
    }

    -- Export JSON report to the active session folder
    self.logger:log_json(result)

    -- Log summary details
    print(string.format("=== TEST RESULT: %s (%s) ===", self.test_name, result.verdict))
    print(string.format("  Samples: %d | Time: %.2f s", N, self.elapsed_time))
    print(string.format("  Lat RMSE:  %.4f m (Limit: %.4f m) -> %s", rmse_y, self.thresholds.rmse_y, pass_y_rmse and "OK" or "FAIL"))
    print(string.format("  Drift Max:  %.4f m (Limit: %.4f m) -> %s", max_drift_x, self.thresholds.max_drift_x, pass_drift and "OK" or "FAIL"))
    print(string.format("  Ori RMSE:  %.4f rad (Limit: %.4f rad) -> %s", rmse_theta, self.thresholds.rmse_theta, pass_theta_rmse and "OK" or "FAIL"))
    print("==========================================")

    return result
end

return LateralTester
