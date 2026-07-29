local robotId = 0
local targetRobotId = 1
local team = 0

require("utils.bangbang")

-- Configuración simple del controlador
local controller = BangBang.new(2.0, 2.0)

-- Posiciones iniciales para grSim
local init_x, init_y = -0.5, -0.5
local target_x, target_y = 1.0, 1.0

grsim.teleport_robot(targetRobotId, team, target_x, target_y, 0.0)
grsim.teleport_robot(robotId, team, init_x, init_y, 0.0)
grsim.teleport_ball(0.0, 0.0)

function process()
    -- 1) Recibe el estado actual del robot y la pelota desde el motor
    local state = get_robot_state(robotId, team)
    local ball = get_ball_state()

    if state and state.active then
        local robot = {
            pos = { x = state.x, y = state.y },
            vel = { x = state.vel_x, y = state.vel_y },
            orientation = state.orientation,
        }

        -- 2) Define un objetivo simple basado en la pelota
        local target = {
            x = ball.x,
            y = ball.y,
        }

        local path = { target }
        local cmd = controller:compute_motion(robot, path, 1.0 / 60.0)

        -- 3) Envía el comando al pipeline de control, que luego lo manda a grSim
        send_velocity(robotId, team, cmd.x, cmd.y, 0.0)

        print(string.format("state=(%.3f, %.3f) target=(%.3f, %.3f) cmd=(%.3f, %.3f)",
            state.x, state.y, target.x, target.y, cmd.x, cmd.y))
    end
end