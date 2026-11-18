-- controller_bench/controllers/fault-aware.lua — a go-to-pose controller meant to keep
-- working on robots with faults (a weak or asymmetric wheel, low torque) and with delay.
--
-- 1. Latency compensation. Vision is old and commands act late; the controller assumes
--    one round-trip delay LATENCY (assumption: 66 ms = 33 ms vision + 33 ms radio). It
--    predicts the pose for when this tick's command acts by replaying the robot-frame
--    commands it already sent over that delay, starting from the vision pose and velocity,
--    with a first-order model (velocity moves toward each command by at most a·dt).
-- 2. Go-to-pose on the predicted state: wanted speed toward the target is
--    min(v_max, sqrt(2·a_brake·d), KP·d) — the sqrt braking curve far away, a linear
--    (P) zone near the target so delay and noise don't make it hunt. Heading the same.
-- 3. The command ramps by at most A_RAMP·dt per tick, and is never allowed to run more
--    than MAX_LEAD ahead of the predicted velocity: a weak robot is not asked for speed
--    it can't reach, so it doesn't brake too late.
-- 4. The command is turned into the robot frame with the heading predicted for when it acts.
--
-- It reads only vision (get_robot_state) and its own command history.

local M = {}

M.DEFAULTS = {
    v_max = 2.5,      -- m/s
    a_max = 2.5,      -- m/s²
    omega_max = 6.0,  -- rad/s
    alpha_max = 20.0, -- rad/s²
    dt = 1 / 60,      -- s, control period
}

local LATENCY = 0.066      -- s, assumed vision + radio delay
local KP = 4.0             -- 1/s, position gain of the linear zone
local KTH = 6.0            -- 1/s, heading gain of the linear zone
local BRAKE_MARGIN = 0.85  -- plan the stop with this fraction of the acceleration
local MAX_LEAD = 0.3       -- m/s, most the command may be ahead of the predicted velocity
local W_LEAD = 1.5         -- rad/s, same for the turn rate

local function wrap(a)
    while a > math.pi do a = a - 2 * math.pi end
    while a <= -math.pi do a = a + 2 * math.pi end
    return a
end

local function approach2(from_x, from_y, to_x, to_y, step)
    local dx, dy = to_x - from_x, to_y - from_y
    local d = math.sqrt(dx * dx + dy * dy)
    if d <= step or d == 0 then return to_x, to_y end
    return from_x + dx / d * step, from_y + dy / d * step
end

local function approach1(from, to, step)
    if to > from + step then return from + step end
    if to < from - step then return from - step end
    return to
end

function M.new(params)
    local p = {}
    for k, v in pairs(M.DEFAULTS) do p[k] = (params and params[k]) or v end
    local self = { params = p, st = {} }
    local n_lat = math.max(0, math.floor(LATENCY / p.dt + 0.5))

    function self:reset()
        self.st = {}
    end

    --- Pose and velocity (field frame) when this tick's command will act.
    local function predict(s, r)
        local x, y, th = r.x, r.y, r.orientation
        local vx, vy, w = r.vel_x, r.vel_y, r.omega
        local a_step, al_step = p.a_max * p.dt, p.alpha_max * p.dt
        for i = 1, #s.hist do
            local c = s.hist[i]
            local cs, sn = math.cos(th), math.sin(th)
            local cx, cy = cs * c.vt - sn * c.vn, sn * c.vt + cs * c.vn
            vx, vy = approach2(vx, vy, cx, cy, a_step * 1.5)
            w = approach1(w, c.w, al_step * 1.5)
            x, y, th = x + vx * p.dt, y + vy * p.dt, th + w * p.dt
        end
        return x, y, th, vx, vy, w
    end

    function self:step(id, team, target)
        local r = get_robot_state(id, team)
        if not r or not r.active then return end
        local key = team * 100 + id
        local s = self.st[key]
        if not s then
            s = { hist = {}, vx = r.vel_x, vy = r.vel_y, w = r.omega }
            self.st[key] = s
        end

        local x, y, th, pvx, pvy, pw = predict(s, r)

        -- Translation.
        local ex, ey = target.x - x, target.y - y
        local d = math.sqrt(ex * ex + ey * ey)
        local a_brake = BRAKE_MARGIN * p.a_max
        local speed = math.min(p.v_max, math.sqrt(2 * a_brake * d), KP * d)
        local wx, wy = 0, 0
        if d > 1e-5 then wx, wy = ex / d * speed, ey / d * speed end
        -- Base: last command, held within MAX_LEAD of the predicted velocity.
        local bx, by = approach2(pvx, pvy, s.vx, s.vy, MAX_LEAD)
        local vx, vy = approach2(bx, by, wx, wy, p.a_max * p.dt)

        -- Rotation.
        local tth = target.theta or th
        local eth = wrap(tth - th)
        local w_wanted = math.min(p.omega_max, math.sqrt(2 * BRAKE_MARGIN * p.alpha_max * math.abs(eth)),
            KTH * math.abs(eth))
        if eth < 0 then w_wanted = -w_wanted end
        local bw = approach1(pw, s.w, W_LEAD)
        local w = approach1(bw, w_wanted, p.alpha_max * p.dt)

        s.vx, s.vy, s.w = vx, vy, w

        -- Field → robot frame, with the heading at the middle of the tick the command acts.
        local thc = th + w * p.dt * 0.5
        local c, sn = math.cos(thc), math.sin(thc)
        local vt, vn = c * vx + sn * vy, -sn * vx + c * vy
        table.insert(s.hist, { vt = vt, vn = vn, w = w })
        if #s.hist > n_lat then table.remove(s.hist, 1) end
        send_velocity(id, team, vt, vn, w)
    end

    return self
end

return M
