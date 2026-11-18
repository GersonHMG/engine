-- controller_bench/controllers/bang_bang.lua — the baseline: a time-optimal
-- (bang-bang) go-to-pose controller written in Lua, sending raw velocities.
--
-- Translation, in field coordinates:
--   wanted speed  = min(v_max, sqrt(2 · a_max · distance))   along the error
--   command       = last command moved toward the wanted velocity by at most a_max · dt
-- so the robot accelerates at a_max, cruises at v_max and brakes at a_max to stop on the
-- point (the sqrt curve is the braking curve). Rotation does the same on the heading
-- error with omega_max / alpha_max. The command is turned into the robot's frame with
-- the measured heading and sent with send_velocity.
--
-- It knows nothing about the robot it drives (torque, grip, a weak wheel): it always plans
-- with the same limits. It reads the state from vision (get_robot_state), not the truth.
-- That is what a better controller should improve on.
--
-- A controller is created with new(params) and has:
--   :step(id, team, target)  once per tick; target = {x, y, theta}; sends the command
--   :reset()                 forget the last command (call when the robot is teleported)

local M = {}

M.DEFAULTS = {
    v_max = 2.5,      -- m/s
    a_max = 2.5,      -- m/s²
    omega_max = 6.0,  -- rad/s
    alpha_max = 20.0, -- rad/s²
    dt = 1 / 60,      -- s, control period
}

local function wrap(a)
    while a > math.pi do a = a - 2 * math.pi end
    while a <= -math.pi do a = a + 2 * math.pi end
    return a
end

--- Move `from` toward `to` by at most `step` (2D).
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
    local self = { params = p, last = {} }

    function self:reset()
        self.last = {}
    end

    function self:step(id, team, target)
        local r = get_robot_state(id, team)
        if not r or not r.active then return end
        local key = team * 100 + id
        local last = self.last[key] or { vx = r.vel_x, vy = r.vel_y, w = r.omega }

        -- Translation.
        local ex, ey = target.x - r.x, target.y - r.y
        local d = math.sqrt(ex * ex + ey * ey)
        local speed = math.min(p.v_max, math.sqrt(2 * p.a_max * d))
        local wx, wy = 0, 0
        if d > 1e-4 then wx, wy = ex / d * speed, ey / d * speed end
        local vx, vy = approach2(last.vx, last.vy, wx, wy, p.a_max * p.dt)

        -- Rotation.
        local eth = wrap((target.theta or r.orientation) - r.orientation)
        local w_wanted = math.min(p.omega_max, math.sqrt(2 * p.alpha_max * math.abs(eth)))
        if eth < 0 then w_wanted = -w_wanted end
        local w = approach1(last.w, w_wanted, p.alpha_max * p.dt)

        self.last[key] = { vx = vx, vy = vy, w = w }

        -- Field → robot frame.
        local c, s = math.cos(r.orientation), math.sin(r.orientation)
        send_velocity(id, team, c * vx + s * vy, -s * vx + c * vy, w)
    end

    return self
end

return M
