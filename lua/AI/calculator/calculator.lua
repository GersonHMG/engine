local bot_distance_to_ball = require("calculator.bot_distance_to_ball")
local bot_nearest_to_ball        = require("calculator.bot_nearest_to_ball")
local M = {}

-- 0d state WORLD

-- 1d stage
ENEMY_TEAM_BOT_BALL_DISTANCE = {}
ALLY_TEAM_BOT_BALL_DISTANCE = {}

-- 2d stage
NEAREST_BOT_TO_BALL = nil

--- Computes both teams' robots ranked by distance to the ball and stores them
--- in the globals `ally_team_bot_ball_distance` / `enemy_team_bot_ball_distance`.
--- @param team integer  -- your (ally) team id (0 = blue, 1 = yellow)
function M.calc(team)
	local enemy_team = (team == 0) and 1 or 0

	-- 1d stage
	ALLY_TEAM_BOT_BALL_DISTANCE = bot_distance_to_ball.ordered_by_distance_to_ball(team)
	ENEMY_TEAM_BOT_BALL_DISTANCE = bot_distance_to_ball.ordered_by_distance_to_ball(enemy_team)

	-- 2d stage
	NEAREST_BOT_TO_BALL = bot_nearest_to_ball.calc(ENEMY_TEAM_BOT_BALL_DISTANCE, ALLY_TEAM_BOT_BALL_DISTANCE)

end

return M