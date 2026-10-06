-- Robot profile for the engine's built-in simulator (simulator.toml): the values the AI
-- used before profiles existed (hand-set, not measured; see sim_measured.lua).
-- What our robots and the ball can do. The AI and the skills read every physical
-- assumption from the active profile (utils/robot_profile.lua), so a profile measured
-- on real robots (lua/tests/calibrate_profile.lua) changes their decisions, not the code.
-- Units: m, s, rad.
return {
    name = "sim",
    robot = {
        radius = 0.09,             -- m, body radius
        front_distance = 0.073,    -- m, center to the flat kicker face
        kicker_half_width = 0.05,  -- m, half width of the kicker face where the ball can be kicked
        cruise_speed = 2.0,        -- m/s, speed once moving
        turn_rate = 4.0,           -- rad/s, turn rate once turning
        move_overhead = 0.0,       -- s, fixed cost of a move (start and stop): time = overhead + d/speed + turn/rate
        max_accel = 6.0,            -- m/s^2, copied (simulator.toml): braking before reversing
    },
    ball = {
        radius = 0.0215,           -- m
        slide_decel = 3.0,         -- m/s^2, while sliding right after a kick
        roll_decel = 1.0,          -- m/s^2, once rolling (from 5/7 of the kick speed down)
    },
    kick = {
        speed = 3.0,               -- m/s, flat kick (kickx)
        reach = 0.01,              -- m, how far in front of the kicker face a kick still fires
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
