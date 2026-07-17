local TrajectoryTester = {}
TrajectoryTester.__index = TrajectoryTester

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

-- Helper to compute minimum distance (deviation) to the path
local function get_deviation_from_path(pos, path_points)
    local min_dist = math.huge
    local closest_ref = nil
    for _, p in ipairs(path_points) do
        local dx = p.x - pos.x
        local dy = p.y - pos.y
        local dist = math.sqrt(dx * dx + dy * dy)
        if dist < min_dist then
            min_dist = dist
            closest_ref = p
        end
    end
    return min_dist, closest_ref
end

-- Constructor
function TrajectoryTester.new(test_name, config)
    local self = setmetatable({}, TrajectoryTester)
    self.test_name = test_name
    config = config or {}
    self.robot_id = config.robot_id or 0
    self.team = config.team or 0
    self.thresholds = config.thresholds or {
        rmse_pos = 0.05,        -- metros
        rmse_theta = 0.1,       -- radianes
        max_err_pos = 0.1,      -- metros
        max_err_theta = 0.2,    -- radianes
        arrival_tolerance = 0.05, -- metros (tolerancia para finalizar el test al llegar al último punto)
    }
    self.timeout_failed = false
    self.trajectory = nil
    self.samples = {}
    self.elapsed_time = 0
    self.finished = false
    self.logger = nil
    return self
end

-- Start a trajectory test
function TrajectoryTester:start(trajectory)
    self.trajectory = trajectory
    self.samples = {}
    self.elapsed_time = 0
    self.finished = false
    self.timeout_failed = false
    
    -- Pre-sample trajectory points to compute geometric path deviation (cross-track error)
    self.path_points = {}
    local step = 0.01
    local t = 0
    while t <= trajectory.total_time do
        table.insert(self.path_points, trajectory.get_pose(t))
        t = t + step
    end
    table.insert(self.path_points, trajectory.get_pose(trajectory.total_time))
    
    -- Teleport ball far away to avoid interference
    grsim.teleport_ball(5.0, 5.0)
    
    -- Teleport robot to start position
    local start_ref = self.trajectory.get_pose(0)
    grsim.teleport_robot(self.robot_id, self.team, start_ref.x, start_ref.y, start_ref.theta)
    
    -- Initialize logger with main = true
    self.logger = Logger.new(self.test_name, {
        "ref_x", "ref_y", "ref_theta",
        "act_x", "act_y", "act_theta",
        "err_pos", "err_theta"
    }, true)
end

-- Main loop tick update (called at 60 Hz)
function TrajectoryTester:update()
    if self.finished then
        return true
    end
    
    local state = get_robot_state(self.robot_id, self.team)
    if not state or not state.active then
        return false
    end
    
    -- Update elapsed time
    self.elapsed_time = self.elapsed_time + (1.0 / 60.0)
    
    -- Get current reference pose
    local ref = self.trajectory.get_pose(self.elapsed_time)
    
    -- Calculate errors based on spatial deviation from the closest path point
    local err_pos, closest_ref = get_deviation_from_path(state, self.path_points)
    local err_theta = math.abs(normalize_angle(closest_ref.theta - state.orientation))
    
    -- Record in telemetry
    self.logger:log_csv({
        ref_x = closest_ref.x,
        ref_y = closest_ref.y,
        ref_theta = closest_ref.theta,
        act_x = state.x,
        act_y = state.y,
        act_theta = state.orientation,
        err_pos = err_pos,
        err_theta = err_theta
    })
    
    -- Accumulate for final statistics
    table.insert(self.samples, { err_pos = err_pos, err_theta = err_theta })
    
    -- Issue movement command to physical system
    move_to(self.robot_id, self.team, {x = ref.x, y = ref.y})
    
    -- Orient the robot towards target orientation
    local face_pt = {
        x = state.x + math.cos(ref.theta),
        y = state.y + math.sin(ref.theta)
    }
    face_to(self.robot_id, self.team, face_pt)
    
    -- Visual helper drawings
    highlight_robot(self.robot_id, self.team)
    draw_point(ref.x, ref.y, {0.0, 1.0, 0.0}) -- Green reference point
    draw_text(ref.x + 0.1, ref.y + 0.1, string.format("t=%.2fs", self.elapsed_time), {1.0, 1.0, 1.0})
    
    -- Check if trajectory completed
    local time_elapsed_fully = self.elapsed_time >= self.trajectory.total_time
    
    if time_elapsed_fully then
        local final_ref = self.trajectory.get_pose(self.trajectory.total_time)
        local dx_final = final_ref.x - state.x
        local dy_final = final_ref.y - state.y
        local dist_to_final = math.sqrt(dx_final * dx_final + dy_final * dy_final)
        
        local arrival_tol = self.thresholds.arrival_tolerance or 0.05
        local arrived = dist_to_final <= arrival_tol
        local timeout_exceeded = self.elapsed_time >= (self.trajectory.total_time + 5.0)
        
        if arrived or timeout_exceeded then
            self.finished = true
            if timeout_exceeded and not arrived then
                self.timeout_failed = true
                print(string.format("[TEST] TIMEOUT EXCEEDED: Robot failed to reach final waypoint (distance to final: %.4fm, tolerance: %.4fm)", dist_to_final, arrival_tol))
            else
                print(string.format("[TEST] ARRIVED: Robot reached final waypoint (distance: %.4fm)", dist_to_final))
            end
            self:evaluate()
            return true
        end
    end
    
    return false
end

-- Calculate statistics and write JSON report
function TrajectoryTester:evaluate()
    local N = #self.samples
    if N == 0 then
        return { verdict = "FAIL", reason = "No samples gathered" }
    end
    
    local sum_err_pos = 0
    local sum_sq_err_pos = 0
    local max_err_pos = 0
    
    local sum_err_theta = 0
    local sum_sq_err_theta = 0
    local max_err_theta = 0
    
    for _, s in ipairs(self.samples) do
        sum_err_pos = sum_err_pos + s.err_pos
        sum_sq_err_pos = sum_sq_err_pos + (s.err_pos * s.err_pos)
        if s.err_pos > max_err_pos then
            max_err_pos = s.err_pos
        end
        
        sum_err_theta = sum_err_theta + s.err_theta
        sum_sq_err_theta = sum_sq_err_theta + (s.err_theta * s.err_theta)
        if s.err_theta > max_err_theta then
            max_err_theta = s.err_theta
        end
    end
    
    local mae_pos = sum_err_pos / N
    local rmse_pos = math.sqrt(sum_sq_err_pos / N)
    
    local mae_theta = sum_err_theta / N
    local rmse_theta = math.sqrt(sum_sq_err_theta / N)
    
    -- Check thresholds
    local pass_pos_rmse = rmse_pos <= self.thresholds.rmse_pos
    local pass_pos_max = max_err_pos <= self.thresholds.max_err_pos
    local pass_theta_rmse = rmse_theta <= self.thresholds.rmse_theta
    local pass_theta_max = max_err_theta <= self.thresholds.max_err_theta
    local pass_arrival = not self.timeout_failed
    
    local pass = pass_pos_rmse and pass_pos_max and pass_theta_rmse and pass_theta_max and pass_arrival
    
    local result = {
        test_name = self.test_name,
        verdict = pass and "PASS" or "FAIL",
        metrics = {
            samples = N,
            rmse_position = rmse_pos,
            mae_position = mae_pos,
            max_error_position = max_err_pos,
            rmse_orientation = rmse_theta,
            mae_orientation = mae_theta,
            max_error_orientation = max_err_theta,
            execution_time_seconds = self.elapsed_time,
            arrived = pass_arrival
        },
        thresholds = self.thresholds
    }
    
    -- Export to JSON report
    self.logger:log_json(result)
    
    -- Output results to engine logs
    print(string.format("=== TEST RESULT: %s (%s) ===", self.test_name, result.verdict))
    print(string.format("  Samples: %d | Time: %.2f s", N, self.elapsed_time))
    print(string.format("  Pos RMSE:  %.4f m (Limit: %.4f m) -> %s", rmse_pos, self.thresholds.rmse_pos, pass_pos_rmse and "OK" or "FAIL"))
    print(string.format("  Pos Max:   %.4f m (Limit: %.4f m) -> %s", max_err_pos, self.thresholds.max_err_pos, pass_pos_max and "OK" or "FAIL"))
    print(string.format("  Ori RMSE:  %.4f rad (Limit: %.4f rad) -> %s", rmse_theta, self.thresholds.rmse_theta, pass_theta_rmse and "OK" or "FAIL"))
    print(string.format("  Ori Max:   %.4f rad (Limit: %.4f rad) -> %s", max_err_theta, self.thresholds.max_err_theta, pass_theta_max and "OK" or "FAIL"))
    print("==========================================")
    
    return result
end

return TrajectoryTester
