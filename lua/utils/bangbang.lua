local math_sqrt = math.sqrt
local math_abs = math.abs
local math_cos = math.cos
local math_sin = math.sin
local math_pi = math.pi 

-- ----------------------------------------------------------------------------
-- 1. EVALUACIÓN DE TRAYECTORIA 1D (Tiempo exacto y velocidad inmediata)
-- ----------------------------------------------------------------------------
-- @param a_max number Aceleración máxima permitida en este eje
-- @param v_max number Velocidad máxima permitida en este eje
-- @param v0 number    Velocidad inicial actual
-- @param wf number    Distancia al objetivo (w_final - w_actual)
-- @return number tf   Tiempo total para llegar a destino con velocidad 0
-- @return number v_next Velocidad objetivo de comando para este ciclo

local function evaluate_1d(a_max, v_max, v0, wf)
    if math_abs(wf) < 0.0001 and math_abs(v0) < 0.001 then
        return 0.0, 0.0
    end

    -- Normalize the direction of motion
    local sign = 1.0
    if wf < 0.0 then
        sign = -1.0
        wf = -wf
        v0 = -v0
    end
    
    local t_total = 0.0
    local v = v0
    local d = wf
    local v_next = 0.0


    -- Case 1: negative initial velocity
    if v < 0.0 then
        local t1 = -v / a_max
        local d1 = (v * v) / (2.0 * a_max)
        t_total = t_total + t1
        d = d - d1
        v = 0.0
        v_next = v0 + a_max * sign
    end
    
    -- Case 3: initial velocity greater than maximum allowed
    if v > v_max then
        local t3 = (v - v_max) / a_max
        local d3 = (v * v - v_max * v_max) / (2.0 * a_max)
        t_total = t_total + t3
        d = d - d3
        v = v_max
        v_next = v0 - a_max * sign
    end

    -- Case 2.x: movement to the taget with initial velocity within limits
    local d_brake = (v * v) / (2.0 * a_max)

    if d <= d_brake then
        -- Case 2.3: need to brake immediately
        t_total = t_total + (v / a_max)
        v_next = v0 - a_max * sign
    else
        local v_peak = math_sqrt(d * a_max + (v * v) / 2.0)

        if v_peak <= v_max then
            -- Case 2.1: triangular profile
            local t_accel = (v_peak - v) / a_max
            local t_decel = v_peak / a_max
            t_total = t_total + t_accel + t_decel
            v_next = (v < v_peak) and (v0 + a_max * sign) or (v0 - a_max * sign)
        else
            -- Case 2.2: trapezoidal profile
            local t_accel = (v_max - v) / a_max
            local d_accel = (v_max * v_max - v * v) / (2.0 * a_max)
            local d_decel = (v_max * v_max) / (2.0 * a_max)
            local d_cruise = d - d_accel - d_decel
            local t_cruise = d_cruise / v_max
            local t_decel = v_max / a_max

            t_total = t_total + t_accel + t_cruise + t_decel
            v_next = (v < v_max) and (v0 + a_max * sign) or (v_max * sign)
        end
    end

    return t_total, v_next
end

-- ----------------------------------------------------------------------------
-- 2. TRAYECTORIA 2D (Sincronización de ejes mediante Bisección de Alpha)
-- ----------------------------------------------------------------------------
-- @param a_max number Aceleración máxima total del vehículo (cilindro)
-- @param v_max number Velocidad máxima del vehículo
-- @param vx0 number, vy0 number Velocidad global actual (X, Y)
-- @param from_x number, from_y number Posición actual (X, Y)
-- @param to_x number, to_y number     Posición objetivo (X, Y)
-- @return number vx, vy Velocidad global comandada para el siguiente ciclo

local function get_trayectory_2d_velocity(a_max, v_max, vx0, vy0, from_x, from_y, to_x, to_y)
    local wfx = to_x - from_x
    local wfy = to_y - from_y

    local min_alpha = 0.0
    local max_alpha = math_pi / 2.0
    local mid_alpha = 0.0
    local epsilon = 0.05

    local best_vx, best_vy = 0.0, 0.0

    for _ = 1, 15 do
        mid_alpha = (min_alpha + max_alpha) *0.5
        local cos_a = math_cos(mid_alpha)
        local sin_a = math_sin(mid_alpha)   

        local tf_x, vx_next = evaluate_1d(a_max * cos_a, v_max * cos_a, vx0, wfx)
        local tf_y, vy_next = evaluate_1d(a_max * sin_a, v_max * sin_a, vy0, wfy)

        best_vx, best_vy = vx_next, vy_next

        if tf_x == 0.0 or tf_y == 0.0 or math_abs(tf_x - tf_y) < epsilon then
            break
        end

        if tf_x > tf_y then
            max_alpha = mid_alpha
        else
            min_alpha = mid_alpha
        end
    end
    return best_vx, best_vy
end

-- ------------------------------------------------------------
-- Controlador Bang-Bang
-- ------------------------------------------------------------

local BangBangControl = {}
BangBangControl.__index = BangBangControl

function BangBangControl.new(a_max, v_max)
    return setmetatable({a_max = a_max, v_max = v_max}, BangBangControl)
end

function BangBangControl:is_near_to_brake(rx, ry, vx, vy, target_x, target_y)
    local dx_brake = (vx * vx) / (2.0 * self.a_max)
    local dy_brake = (vy * vy) / (2.0 * self.a_max)
    return dx_brake >= math_abs(rx - target_x) or dy_brake >= math_abs(ry - target_y)
end

function BangBangControl:compute_motion(robot, path, delta)
    if delta < (1.0 / 60.0) or #path == 0 then
        return {x = 0.0, y = 0.0, w = 0.0}
    end

    local goal = table.remove(path, 1)

    if #path > 0 and self:is_near_to_brake(robot.pos.x, robot.pos.y, robot.vel.x, robot.vel.y, goal.x, goal.y) then
        goal = table.remove(path, 1)
    end

    local global_vx, global_vy = get_trayectory_2d_velocity(
        self.a_max, self.v_max,
        robot.vel.x, robot.vel.y,
        robot.pos.x, robot.pos.y,
        goal.x, goal.y
    )

    local orientation = robot.orientation
    local cos_theta = math_cos(-orientation)
    local sin_theta = math_sin(-orientation)

    local local_vx = global_vx * cos_theta - global_vy * sin_theta
    local local_vy = global_vx * sin_theta + global_vy * cos_theta

    return {x = local_vx, y = local_vy, w = 0.0}
end

-- -------------------------------------------------------------
-- Estructurar
-- -------------------------------------------------------------

-- Instanciar controlador con limites fisicos
local controller = BangBangControl.new(3.5, 2.0)

-- @param pointA table Posición inicial {x, y}
-- @param pointB table Posición objetivo {x, y}
-- @param robot  table Estado del robot {pos={x,y}, vel={x,y}, orientation=0.0}
function bangBangTrajectory(pointA, pointB, robot)
    local path = { {x = pointB.x, y = pointB.y} }
    
    return controller:compute_motion(robot, path, 1.0 / 60.0)
end

function send_velocity()
    
end

function process()
    
end