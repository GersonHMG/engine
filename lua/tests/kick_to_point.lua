-- Benchmark for skills/kick_to_point: time from the first call until the kick,
-- and how accurately the ball heads to the target.
-- Run: ./target/debug/engine --headless lua/tests/kick_to_point.lua
package.path = "lua/?.lua;" .. package.path
local skill = require("skills.kick_to_point")

local TIME_LIMIT = 10      -- s per setup
local MEASURE_DELAY = 0.3  -- s after the kick to measure the ball direction
local NEAR_DIST = 0.25     -- m, robot-ball distance counted as "reached the ball"

local D45 = 4.5 * math.cos(math.pi / 4)
local setups = {
    { name = "aligned_close", robot = { x = -0.3, y = 0.0 }, target = { x = 4.5, y = 0 } },
    { name = "aligned_far",   robot = { x = -2.0, y = 0.0 }, target = { x = 4.5, y = 0 } },
    { name = "offset_close",  robot = { x = -0.3, y = 0.2 }, target = { x = 4.5, y = 0 } },
    { name = "offset_far",    robot = { x = -1.5, y = 0.8 }, target = { x = 4.5, y = 0 } },
    { name = "angled_target", robot = { x = -1.0, y = 0.0 }, target = { x = D45, y = D45 } },
}

local current, start, near_time, kick, ball_at_kick, prev_ball
local results = {}

local function wrap(a)
    while a > math.pi do a = a - 2 * math.pi end
    while a < -math.pi do a = a + 2 * math.pi end
    return a
end

local function begin(index)
    current = index
    local setup = setups[index]
    -- Face the ball, at rest.
    grsim.teleport_robot(0, 0, setup.robot.x, setup.robot.y, math.atan(-setup.robot.y, -setup.robot.x))
    grsim.teleport_ball(0, 0)
    sim.events() -- drop the events of the previous setup
    start = sim.time()
    near_time, kick, ball_at_kick = nil, nil, nil
    prev_ball = { x = 0, y = 0 }
end

local function fmt(v, f)
    return v and string.format(f, v) or "-"
end

local function next_setup(r)
    r.name = setups[current].name
    results[#results + 1] = r
    print(string.format("%-14s kicked=%-5s t_kick=%6s s  t_near=%6s s  speed=%6s m/s  angle_err=%7s rad",
        r.name, tostring(r.kicked), fmt(r.t_kick, "%.3f"), fmt(r.t_near, "%.3f"),
        fmt(r.speed, "%.2f"), fmt(r.err, "%.4f")))
    if current == #setups then
        local total, kicked = 0, 0
        for _, x in ipairs(results) do
            if x.kicked then
                kicked = kicked + 1
                total = total + x.t_kick
            else
                total = total + TIME_LIMIT
            end
        end
        print(string.format("SUMMARY kicked=%d/%d total_time=%.3f s (no kick counts as %.0f s)",
            kicked, #results, total, TIME_LIMIT))
        sim.finish(true)
    else
        begin(current + 1)
    end
end

function process()
    -- The world snapshot is empty on the first tick of a run: set up there,
    -- and start measuring from the next tick.
    if not current then
        return begin(1)
    end
    local setup = setups[current]
    local now = sim.time()

    if kick then
        -- Kick done: leave the robot idle and wait to measure the ball direction.
        if now - kick.time >= MEASURE_DELAY then
            local ball = get_ball_state()
            local heading = math.atan(ball.vel_y, ball.vel_x)
            local wanted = math.atan(setup.target.y - ball_at_kick.y, setup.target.x - ball_at_kick.x)
            next_setup({
                kicked = true,
                t_kick = kick.time - start,
                t_near = near_time,
                speed = kick.speed,
                err = math.abs(wrap(heading - wanted)),
            })
        end
        return
    end

    local robot = get_robot_state(0, 0)
    local ball = get_ball_state()
    if not near_time and math.sqrt(robot.x ^ 2 + robot.y ^ 2 - 2 * (robot.x * ball.x + robot.y * ball.y) + ball.x ^ 2 + ball.y ^ 2) <= NEAR_DIST then
        near_time = now - start
    end

    skill.process(0, 0, setup.target)

    for _, event in ipairs(sim.events()) do
        if event.kind == "kick" and event.id == 0 and event.team == 0 then
            kick = event
            -- The kick fired during the step after the previous tick's reading.
            ball_at_kick = prev_ball
            return
        end
    end

    prev_ball = { x = ball.x, y = ball.y }

    if now - start > TIME_LIMIT then
        next_setup({ kicked = false, t_near = near_time })
    end
end
