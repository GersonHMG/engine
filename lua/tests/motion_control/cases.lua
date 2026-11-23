-- motion_control/cases.lua — the fixed set of moves the motion-control benchmark runs.
--
-- Every case starts with blue robot 0 at rest at `start` = {x, y, theta}. The target is
--   target   = {x, y}           a fixed point, or
--   target   = function(t)      a point that moves (t = seconds since the case started),
--   switch   = {at = s, target = {x, y}}  the fixed target changes after `at` seconds
--                                (the robot is already moving when it gets the new one),
--   heading  = rad              the heading to hold or turn to (default: start heading).
-- `kind` picks the metrics: "point" (reach and stop on a point) or "track" (follow a
-- moving target).
--
-- Keep this list fixed: results are only comparable between runs of the same cases.
-- Add new cases at the end, and make a new baseline when you do.

local PI = math.pi

return {
    -- Straight moves along the heading, of increasing length.
    { name = "fwd_0.1m",    kind = "point", start = { -1.0, 0.0, 0 }, target = { x = -0.9, y = 0.0 } },
    { name = "fwd_0.5m",    kind = "point", start = { -1.0, 0.0, 0 }, target = { x = -0.5, y = 0.0 } },
    { name = "fwd_1m",      kind = "point", start = { -1.0, 0.0, 0 }, target = { x = 0.0, y = 0.0 } },
    { name = "fwd_2m",      kind = "point", start = { -1.0, 0.0, 0 }, target = { x = 1.0, y = 0.0 } },
    { name = "fwd_4m",      kind = "point", start = { -2.0, 0.0, 0 }, target = { x = 2.0, y = 0.0 } },
    -- Other directions relative to the robot (the robot keeps facing +x).
    { name = "back_1m",     kind = "point", start = { 0.5, 0.0, 0 },  target = { x = -0.5, y = 0.0 } },
    { name = "side_1m",     kind = "point", start = { 0.0, -0.5, 0 }, target = { x = 0.0, y = 0.5 } },
    { name = "diag_2m",     kind = "point", start = { -1.0, -1.0, 0 }, target = { x = 0.414, y = 0.414 } },
    -- Turning: in place, and while driving.
    { name = "turn_90",     kind = "point", start = { 0.0, 0.0, 0 },  target = { x = 0.0, y = 0.0 }, heading = PI / 2 },
    { name = "turn_180",    kind = "point", start = { 0.0, 0.0, 0 },  target = { x = 0.0, y = 0.0 }, heading = PI },
    { name = "fwd_2m_turn", kind = "point", start = { -1.0, 0.0, 0 }, target = { x = 1.0, y = 0.0 }, heading = PI },
    -- Stopping on a point while already moving fast: the target jumps closer.
    { name = "stop_moving", kind = "point", start = { -2.0, 0.0, 0 }, target = { x = 2.5, y = 0.0 },
      switch = { at = 1.2, target = { x = 0.0, y = 0.0 } } },
    -- Reversing: the target jumps behind the moving robot.
    { name = "reverse",     kind = "point", start = { -1.5, 0.0, 0 }, target = { x = 2.0, y = 0.0 },
      switch = { at = 1.0, target = { x = -1.0, y = 0.5 } } },
    -- Following targets that move (like a ball or a pass receiver).
    { name = "track_line",  kind = "track", start = { -2.0, -1.0, 0 }, duration = 5.0,
      target = function(t) return { x = -2.0 + 0.8 * t, y = -0.5 } end },
    { name = "track_circle", kind = "track", start = { 1.0, 0.0, 0 }, duration = 6.0,
      target = function(t) return { x = math.cos(0.8 * t), y = math.sin(0.8 * t) } end },
}
