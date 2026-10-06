-- screen: stands in an enemy's path to a point it wants to reach, without touching it.
-- Parameters: robotId, team, enemyId, protect {x, y} (m): the point to keep the enemy from
-- (the ball, or where a pass will arrive).
-- Terminates when the robot is on the screen spot (within SPOT_TOLERANCE).
--
-- The screen spot is on the line enemy -> protect, GAP beyond contact from the enemy, so
-- the enemy's path planner has to route around the robot. It never drives into the enemy:
-- the spot keeps at least the contact distance plus GAP (SSL forbids pushing), and when
-- the enemy is already within that distance of the protected point there is nothing to
-- block and the robot holds the spot nearest the line instead of charging in.
local screen = {}

local profile = require("utils.robot_profile")

local GAP = 0.08             -- m, free space kept between the two robot bodies
local SPOT_TOLERANCE = 0.05  -- m, distance to the spot that counts as there
local MIN_LENGTH = 0.001     -- m, guard for divisions by a length

-- Robot to robot center distance with GAP between the bodies.
local function standoff()
    return 2 * profile.robot.radius + GAP
end

--- The screen spot for an enemy at `enemy` going to `protect`, or nil when the enemy is too
--- close to `protect` to be screened.
--- @param enemy { x: number, y: number }
--- @param protect { x: number, y: number }
--- @return { x: number, y: number }|nil
function screen.spot(enemy, protect)
    local dx, dy = protect.x - enemy.x, protect.y - enemy.y
    local length = math.sqrt(dx ^ 2 + dy ^ 2)
    if length < standoff() + profile.touch_distance() then
        return nil
    end
    local k = standoff() / math.max(length, MIN_LENGTH)
    return { x = enemy.x + dx * k, y = enemy.y + dy * k }
end

local function is_done(robot, spot)
    return spot ~= nil and math.sqrt((robot.x - spot.x) ^ 2 + (robot.y - spot.y) ^ 2) <= SPOT_TOLERANCE
end

function screen.process(robotId, team, enemyId, protect)
    local robot = get_robot_state(robotId, team)
    local enemy = get_robot_state(enemyId, 1 - team)
    local spot = screen.spot(enemy, protect)

    if is_done(robot, spot) then
        return true
    end
    if not spot then
        -- Nothing to block: keep facing the enemy where we are.
        face_to(robotId, team, { x = enemy.x, y = enemy.y })
        return false
    end

    draw_point(spot.x, spot.y, true, { r = 1.0, g = 1.0, b = 0.0 })
    -- move_to: the path planner keeps clear of the enemy (and every other robot) on the way.
    move_to(robotId, team, spot)
    face_to(robotId, team, { x = enemy.x, y = enemy.y })
    return false
end

return screen
