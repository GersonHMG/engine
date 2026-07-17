local trajectory_generator = {}

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

-- Linear trajectory
function trajectory_generator.linear(start_p, end_p, duration)
    return {
        total_time = duration,
        get_pose = function(t)
            if t >= duration then
                return {x = end_p.x, y = end_p.y, theta = end_p.theta}
            end
            local u = t / duration
            local d_theta = normalize_angle(end_p.theta - start_p.theta)
            return {
                x = start_p.x + (end_p.x - start_p.x) * u,
                y = start_p.y + (end_p.y - start_p.y) * u,
                theta = normalize_angle(start_p.theta + d_theta * u)
            }
        end
    }
end

-- Circular trajectory
function trajectory_generator.circular(center, radius, start_angle, duration)
    return {
        total_time = duration,
        get_pose = function(t)
            local progress = t / duration
            local angle = start_angle + 2 * math.pi * progress
            local x = center.x + radius * math.cos(angle)
            local y = center.y + radius * math.sin(angle)
            -- Tangential orientation
            local theta = normalize_angle(angle + math.pi / 2)
            return {x = x, y = y, theta = theta}
        end
    }
end

-- Square trajectory
function trajectory_generator.square(corners, duration)
    -- corners is a list of 4 points: {p1, p2, p3, p4}
    -- we close the loop: p1 -> p2 -> p3 -> p4 -> p1
    local pts = {corners[1], corners[2], corners[3], corners[4], corners[1]}
    local segment_duration = duration / 4
    return {
        total_time = duration,
        get_pose = function(t)
            if t >= duration then
                return {x = pts[1].x, y = pts[1].y, theta = pts[1].theta}
            end
            local idx = math.floor(t / segment_duration) + 1
            if idx > 4 then idx = 4 end
            local local_t = t - (idx - 1) * segment_duration
            local u = local_t / segment_duration
            local start_p = pts[idx]
            local end_p = pts[idx + 1]
            local d_theta = normalize_angle(end_p.theta - start_p.theta)
            return {
                x = start_p.x + (end_p.x - start_p.x) * u,
                y = start_p.y + (end_p.y - start_p.y) * u,
                theta = normalize_angle(start_p.theta + d_theta * u)
            }
        end
    }
end

-- S-Curve trajectory (sine wave along a path)
function trajectory_generator.s_curve(start_p, end_p, amplitude, frequency, duration)
    local dx = end_p.x - start_p.x
    local dy = end_p.y - start_p.y
    local distance = math.sqrt(dx*dx + dy*dy)
    local angle = math.atan(dy, dx)
    
    return {
        total_time = duration,
        get_pose = function(t)
            if t >= duration then
                return {x = end_p.x, y = end_p.y, theta = end_p.theta}
            end
            local u = t / duration
            local s = distance * u
            local perp_offset = amplitude * math.sin(2 * math.pi * frequency * u)
            
            local x = start_p.x + s * math.cos(angle) - perp_offset * math.sin(angle)
            local y = start_p.y + s * math.sin(angle) + perp_offset * math.cos(angle)
            
            local deriv_perp = amplitude * (2 * math.pi * frequency) * math.cos(2 * math.pi * frequency * u)
            local vx = distance * math.cos(angle) - deriv_perp * math.sin(angle)
            local vy = distance * math.sin(angle) + deriv_perp * math.cos(angle)
            local theta = normalize_angle(math.atan(vy, vx))
            
            return {x = x, y = y, theta = theta}
        end
    }
end

return trajectory_generator
