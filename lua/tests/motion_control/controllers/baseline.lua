-- motion_control/controllers/baseline.lua — the controller every skill uses today:
-- the engine's move_to (path planner + bang-bang) and face_to (PID, default gains).
--
-- A controller is a function(id, team, target, heading) called once per tick, that sends
-- this tick's command for robot (id, team) to go to `target` = {x, y} and face `heading`
-- (rad). To try a new controller, copy this file to controllers/<name>.lua, change it,
-- and run the benchmark with MC_CONTROLLER=<name> (see ../README.md).

return function(id, team, target, heading)
    local r = get_robot_state(id, team)
    move_to(id, team, target)
    face_to(id, team, { x = r.x + math.cos(heading), y = r.y + math.sin(heading) })
end
