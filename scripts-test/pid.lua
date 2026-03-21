local waypoints = {
    {x = 0.5, y = 0.2},
    {x = 2.0, y = 0.8},
    {x = -1.5, y = 0.3}
}

local current_index = 1
local step = 1
local tolerance = 0.05

local robot_id = 0
local team_id = 0
local dt = 1.0 / 60.0

-- Position PID gains (world frame)
local kp_x = 3.8*10
local ki_x = 0.05
local kd_x = 0.2

local kp_y = 3.8*10
local ki_y = 0.05
local kd_y = 0.2

-- Heading controller gain
local kp_w = 0 --3.0*2

-- Velocity limits
local v_max = 10.0
local w_max = 8.0

local err_x_i = 0.0
local err_y_i = 0.0
local err_x_prev = 0.0
local err_y_prev = 0.0

local function clamp(v, vmin, vmax)
    if v < vmin then return vmin end
    if v > vmax then return vmax end
    return v
end

local function normalize_angle(a)
    while a > math.pi do a = a - 2.0 * math.pi end
    while a < -math.pi do a = a + 2.0 * math.pi end
    return a
end

local function reset_pid_state()
    err_x_i = 0.0
    err_y_i = 0.0
    err_x_prev = 0.0
    err_y_prev = 0.0
end

function process()
    for _, p in ipairs(waypoints) do
        draw_point(p.x, p.y)
    end

    local target = waypoints[current_index]
    local rs = get_robot_state(robot_id, team_id)
    local rx = rs.x
    local ry = rs.y
    local orientation = rs.orientation

    local dx = target.x - rx
    local dy = target.y - ry
    local distance = math.sqrt((dx * dx) + (dy * dy))

    if distance <= tolerance then
        current_index = current_index + step

        if current_index > #waypoints then
            step = -1
            current_index = #waypoints - 1
        elseif current_index < 1 then
            step = 1
            current_index = 2
        end

        target = waypoints[current_index]
        reset_pid_state()

        dx = target.x - rx
        dy = target.y - ry
    end

    -- PID in world frame
    err_x_i = err_x_i + dx * dt
    err_y_i = err_y_i + dy * dt

    local err_x_d = (dx - err_x_prev) / dt
    local err_y_d = (dy - err_y_prev) / dt

    err_x_prev = dx
    err_y_prev = dy

    local vx_world = (kp_x * dx) + (ki_x * err_x_i) + (kd_x * err_x_d)
    local vy_world = (kp_y * dy) + (ki_y * err_y_i) + (kd_y * err_y_d)

    -- Clamp translational speed magnitude
    local speed = math.sqrt((vx_world * vx_world) + (vy_world * vy_world))
    if speed > v_max and speed > 1e-9 then
        local scale = v_max / speed
        vx_world = vx_world * scale
        vy_world = vy_world * scale
    end

    -- Keep robot facing target
    local desired_theta = math.atan(dy, dx)
    local theta_error = normalize_angle(desired_theta - orientation)
    local omega = clamp(kp_w * theta_error, -w_max, w_max)

    -- Convert world-frame velocity to robot local frame for send_velocity
    local c = math.cos(orientation)
    local s = math.sin(orientation)
    local vx_local = (c * vx_world) + (s * vy_world)
    local vy_local = (-s * vx_world) + (c * vy_world)

    send_velocity(robot_id, team_id, vx_local, vy_local, omega)
end