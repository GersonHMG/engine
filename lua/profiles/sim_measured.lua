-- Robot profile measured by lua/tests/calibrate_profile.lua (simulator).
-- Simulator values (simulator.toml), measured 2026-10-06. The turn rate is capped: turns in
-- the simulator take a nearly constant ~0.56 s whatever the angle (the cost is in the overhead).
return {
    name = "sim_measured",
    robot = {
        radius = 0.090,             -- copied (not measured)
        front_distance = 0.073,     -- copied (not measured)
        kicker_half_width = 0.050,  -- copied (not measured)
        cruise_speed = 1.787,      -- m/s, once moving
        turn_rate = 20.000,         -- rad/s, once turning
        move_overhead = 0.575,     -- s, fixed cost of a move (drive fit; turn fit gave 0.556 s)
        max_accel = 6.0,            -- m/s^2, copied (simulator.toml): braking before reversing
    },
    ball = {
        radius = 0.0215,            -- copied (not measured)
        slide_decel = 2.916,       -- m/s^2
        roll_decel = 1.000,        -- m/s^2
    },
    kick = {
        speed = 3.240,             -- m/s
        reach = 0.010,              -- copied (not measured)
        one_touch_angle = 1.0,      -- rad, largest angle between the heading and an incoming ball
                                    -- that can still be kicked on the first touch (geometric estimate)
        angle_error = 0.08,         -- rad, typical direction error of a kick (kick_to_point benchmark, mean)
    },
    -- How far off the AI's time estimates can be, s: the width of the "who gets there first"
    -- probabilities. Larger for robots whose timing varies more (real robots, vision delay).
    timing_noise = 0.3,
    -- The opponents. Motion values they lack are our robot's (cruise_speed, move_overhead,
    -- turn_rate, max_accel). read_passes: how much of our line-up time they use to read a
    -- pass before it is kicked (0 = they follow the ball only, 1 = they cover the receiver
    -- at once). guard_distance (m): ball-following opponents stand this far in front of the
    -- ball, towards their goal (nil: no assumption). Set both from watching the opponent.
    opponent = { read_passes = 0.0, guard_distance = 0.35 },
}
