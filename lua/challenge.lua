-- ==============================================================================
-- Challenge Script: 3 Robots Ball Passing in Triangle Formation
-- ==============================================================================
-- This script implements a state machine for 3 robots to continuously pass a
-- ball while maintaining a triangle formation using existing skills and tactics.
--
-- States: INIT -> FORMATION -> FETCH_BALL -> PASSING (cycle)
-- ==============================================================================
-- Import utilities and tactics
local utils = require("utils.utils")
local has_the_ball = utils.has_the_ball
local getDistance = utils.getDistance

-- Import skills
local go_to_ball = require("skills.go_to_ball")
local move = require("skills.move")
local move_ball = require("skills.move_ball")
local kick_to_point = require("skills.kick_to_point")
local rotate_to_aim = require("skills.rotate_to_aim")   

-- Import tactics
local Pass = require("tactics.active.pass")
local ReceivePass = require("tactics.active.receive_pass")

-- ==============================================================================
-- Configuration
-- ==============================================================================

local CONFIG = {
    TEAM = 0, -- Blue team (0 = blue, 1 = yellow)
    ROBOT_IDS = {0, 1, 2}, -- Robot IDs
    TRIANGLE_RADIUS = 0.8, -- Distance from center to each vertex
    FORMATION_CENTER = {
        x = 0,
        y = 0
    }, -- Center of triangle formation
    BALL_LOSS_DISTANCE = 1.5, -- Max distance before ball is considered lost
    DIST_TOLERANCE = 0.08, -- Tolerance for reaching a position
    ANGLE_TOLERANCE = 0.15, -- Tolerance for facing direction
    PASS_TIMEOUT = 60, -- Frames until we timeout on a pass attempt
    DRIBBLER_SPEED = 5 -- Dribbler motor speed
}

-- ==============================================================================
-- State Machine
-- ==============================================================================

local Challenge = {
    state = "INIT",
    frame = 0,
    current_ball_holder = nil,
    next_receiver = nil,
    pass_start_frame = 0,
    triangle_vertices = {},
    robot_states = {}, -- Track what each robot is doing
    pass_instance = nil,
    receive_instances = {} -- Instances for each receiver
}

-- ==============================================================================
-- Helper Functions (Using External Skills & Tactics)
-- ==============================================================================

--- Calculate triangle vertices based on center and radius
function Challenge.calculate_triangle_vertices(center, radius)
    local vertices = {}
    -- Robot 1: top
    vertices[1] = {
        x = center.x,
        y = center.y + radius
    }
    -- Robot 2: bottom-left
    vertices[2] = {
        x = center.x - (radius * math.sqrt(3) / 2),
        y = center.y - (radius / 2)
    }
    -- Robot 3: bottom-right
    vertices[3] = {
        x = center.x + (radius * math.sqrt(3) / 2),
        y = center.y - (radius / 2)
    }
    return vertices
end

--- Calculate distance between two points
function Challenge.distance(p1, p2)
    local dx = p1.x - p2.x
    local dy = p1.y - p2.y
    return math.sqrt(dx * dx + dy * dy)
end

--- Check if a robot is at a target position
function Challenge.is_at_position(robot_id, target, tolerance)
    local robot_state = get_robot_state(robot_id, CONFIG.TEAM)
    return Challenge.distance(robot_state, target) <= tolerance
end

--- Check if a robot is facing a target point
function Challenge.is_facing(robot_id, target, tolerance)
    local robot_state = get_robot_state(robot_id, CONFIG.TEAM)
    local dx = target.x - robot_state.x
    local dy = target.y - robot_state.y
    local target_angle = math.atan(dy, dx)

    local angle_diff = math.abs(robot_state.orientation - target_angle)
    while angle_diff > math.pi do
        angle_diff = angle_diff - (2 * math.pi)
    end
    angle_diff = math.abs(angle_diff)

    return angle_diff <= tolerance
end

--- Find the robot closest to the ball
function Challenge.find_closest_robot_to_ball()
    local ball_state = get_ball_state()
    local closest_id = nil
    local min_distance = math.huge

    for _, robot_id in ipairs(CONFIG.ROBOT_IDS) do
        local robot_state = get_robot_state(robot_id, CONFIG.TEAM)
        local dist = Challenge.distance(robot_state, ball_state)
        if dist < min_distance then
            min_distance = dist
            closest_id = robot_id
        end
    end

    return closest_id, min_distance
end

--- Get the next receiver in the passing cycle
function Challenge.get_next_receiver(current_holder)
	if current_holder == 0 then
		return 1
	elseif current_holder == 1 then
		return 2
	else
		return 0
	end
end

--- Set robot action status
function Challenge.set_robot_action(robot_id, action)
    Challenge.robot_states[robot_id] = action
end

--- Visualize triangle formation
function Challenge.visualize_triangle()
    if not Challenge.triangle_vertices or #Challenge.triangle_vertices < 3 then
        return
    end

    for i = 1, 3 do
        draw_point(Challenge.triangle_vertices[i].x, Challenge.triangle_vertices[i].y, true, {
            r = 0.0,
            g = 1.0,
            b = 0.0
        })
    end

    draw_line({Challenge.triangle_vertices[1], Challenge.triangle_vertices[2], Challenge.triangle_vertices[3],
               Challenge.triangle_vertices[1]}, true, {
        r = 0.0,
        g = 0.5,
        b = 0.0
    })
end

--- Visualize current state and robot actions
function Challenge.visualize_state()
    local state_text = string.format("State: %s | Frame: %d", Challenge.state, Challenge.frame)
    draw_text(-2.5, 3.0, state_text, {
        r = 1.0,
        g = 1.0,
        b = 0.0
    })

    if Challenge.current_ball_holder then
        draw_text(-2.5, 2.7, string.format("Ball Holder: %d", Challenge.current_ball_holder), {
            r = 1.0,
            g = 0.5,
            b = 0.0
        })
    end

    -- Display pass information
    if Challenge.current_ball_holder and Challenge.next_receiver then
        draw_text(-2.5, 2.4, string.format("Pass: Robot%d -> Robot%d", Challenge.current_ball_holder, Challenge.next_receiver), {
            r = 0.0,
            g = 1.0,
            b = 1.0
        })
    end

    -- Display robot actions above each robot
    for _, robot_id in ipairs(CONFIG.ROBOT_IDS) do
        local robot_state = get_robot_state(robot_id, CONFIG.TEAM)
        local action = Challenge.robot_states[robot_id] or "waiting"
        local color = {
            r = 0.5,
            g = 1.0,
            b = 0.5
        }

        -- Highlight ball holder in different color
        if robot_id == Challenge.current_ball_holder then
            color = {
                r = 1.0,
                g = 1.0,
                b = 0.0
            }
        end

        draw_text(robot_state.x - 0.25, robot_state.y + 0.35, string.format("R%d: %s", robot_id, action), color)
    end

    Challenge.visualize_triangle()
end

-- ==============================================================================
-- State Functions - Using External Skills
-- ==============================================================================

--- State: INIT - Initialize triangle and find ball holder
local function state_init()
    Challenge.triangle_vertices = Challenge.calculate_triangle_vertices(CONFIG.FORMATION_CENTER, CONFIG.TRIANGLE_RADIUS)

    -- Initialize robot states
    for _, robot_id in ipairs(CONFIG.ROBOT_IDS) do
        Challenge.robot_states[robot_id] = "init"
    end

    -- Find closest robot to ball to be the first holder
    local closest_robot = Challenge.find_closest_robot_to_ball()
    Challenge.current_ball_holder = closest_robot
    Challenge.next_receiver = Challenge.get_next_receiver(Challenge.current_ball_holder)

    -- Initialize receive pass instances
    for _, robot_id in ipairs(CONFIG.ROBOT_IDS) do
        Challenge.receive_instances[robot_id] = ReceivePass.new()
    end

    Challenge.state = "FORMATION"
    Challenge.frame = 0
end

--- State: FORMATION - Move robots to triangle vertices
local function state_formation()
    local all_in_formation = true

    for idx, robot_id in ipairs(CONFIG.ROBOT_IDS) do
        local target = Challenge.triangle_vertices[idx]

        Challenge.set_robot_action(robot_id, "formation")

        if not Challenge.is_at_position(robot_id, target, CONFIG.DIST_TOLERANCE) then
            all_in_formation = false
            move.process(robot_id, CONFIG.TEAM, target)
        end
    end

    if all_in_formation then
        Challenge.state = "FETCH_BALL"
        Challenge.frame = 0
    else
        Challenge.frame = Challenge.frame + 1
    end
end

--- State: FETCH_BALL - Use move_ball skill to fetch and move ball to formation vertex
local function state_fetch_ball()
    local closest_robot = Challenge.find_closest_robot_to_ball()
    -- Map robot IDs (0,1,2) to triangle vertices (1,2,3)
    local robot_vertex = Challenge.triangle_vertices[closest_robot + 1]

    Challenge.set_robot_action(closest_robot, "fetch_ball")

    -- Use the move_ball skill to fetch ball and bring it to the robot's own formation vertex
    if move_ball.process(closest_robot, CONFIG.TEAM, robot_vertex) then
        Challenge.current_ball_holder = closest_robot
        Challenge.next_receiver = Challenge.get_next_receiver(Challenge.current_ball_holder)
        Challenge.state = "PASSING"
        Challenge.frame = 0
        Challenge.pass_start_frame = 0
    end

    Challenge.frame = Challenge.frame + 1
end
--- State: PASSING - Execute passing using Pass and ReceivePass tactics
local function state_passing()
    local holder = Challenge.current_ball_holder
    local receiver = Challenge.next_receiver

    -- Check if ball was lost
    local closest_robot, closest_distance = Challenge.find_closest_robot_to_ball()

    if closest_distance > CONFIG.BALL_LOSS_DISTANCE and not has_the_ball(holder, CONFIG.TEAM) then
        -- Ball lost! Return to FETCH_BALL
        Challenge.state = "FETCH_BALL"
        Challenge.frame = 0
        for _, robot_id in ipairs(CONFIG.ROBOT_IDS) do
            Challenge.receive_instances[robot_id]:reset()
        end
        return
    end

    -- Find holder index for triangle vertices
    local holder_idx = 1
    for i, id in ipairs(CONFIG.ROBOT_IDS) do
        if id == holder then
            holder_idx = i
        end
    end

    -- If the holder still has the ball, prepare the pass
    if has_the_ball(holder, CONFIG.TEAM) then
        Challenge.set_robot_action(holder, "passing")

        local holder_vertex = Challenge.triangle_vertices[holder_idx]
        -- Get receiver ID and convert to triangle index (ID + 1)
        local receiver_id = Challenge.get_next_receiver(holder)
        local receiver_vertex = Challenge.triangle_vertices[receiver_id + 1]
        
        -- Move toward position while aiming at receiver simultaneously
        move_to(holder, CONFIG.TEAM, holder_vertex)
        rotate_to_aim.process(holder, CONFIG.TEAM, receiver_vertex, 0.2)

        -- Create a new Pass instance for this pass if not already created
        if not Challenge.pass_instance or Challenge.pass_instance:get_state() == Pass.State.done then
            Challenge.pass_instance = Pass.new()
        end

        -- Use Pass tactic to kick to receiver - use receiver_id (calculated, not stored)
        -- Use Pass tactic to kick to receiver - use receiver_id (calculated, not stored)
        if Challenge.pass_instance:process(holder, CONFIG.TEAM, receiver_id) then
            -- El pase se ha ejecutado (el balón está rodando).
            -- NO actualizamos los roles todavía. Dejamos que el bloque 'else' maneje la recepción.
            Challenge.pass_start_frame = Challenge.frame
        end
    else
        -- Receiver waiting for pass
        Challenge.set_robot_action(receiver, "receive_pass")

        -- Use ReceivePass tactic for interception
        Challenge.receive_instances[receiver]:process(receiver, CONFIG.TEAM)

        -- Check if we successfully received
        if has_the_ball(receiver, CONFIG.TEAM) then
            Challenge.current_ball_holder = receiver
            Challenge.next_receiver = Challenge.get_next_receiver(receiver)
            Challenge.pass_start_frame = Challenge.frame
        end
    end

    -- Other robots wait in formation AND look at the ball
    local ball_pos = get_ball_state()
    for idx, robot_id in ipairs(CONFIG.ROBOT_IDS) do
        if robot_id ~= holder and robot_id ~= receiver then
            Challenge.set_robot_action(robot_id, "waiting")
            local vertex = Challenge.triangle_vertices[idx]
            move_to(robot_id, CONFIG.TEAM, vertex)
            face_to(robot_id, CONFIG.TEAM, ball_pos)
        end
    end

    -- Check for pass timeout
    if Challenge.frame - Challenge.pass_start_frame > CONFIG.PASS_TIMEOUT then
        Challenge.state = "FETCH_BALL"
        Challenge.frame = 0
        for _, robot_id in ipairs(CONFIG.ROBOT_IDS) do
            Challenge.receive_instances[robot_id]:reset()
        end
        if Challenge.pass_instance then
            Challenge.pass_instance:reset()
        end
        return
    end

    Challenge.frame = Challenge.frame + 1
end
-- ==============================================================================
-- Main Process Loop
-- ==============================================================================

function Challenge.process()
    Challenge.visualize_state()

    if Challenge.state == "INIT" then
        state_init()
    elseif Challenge.state == "FORMATION" then
        state_formation()
    elseif Challenge.state == "FETCH_BALL" then
        state_fetch_ball()
    elseif Challenge.state == "PASSING" then
        state_passing()
    end
end

return Challenge
