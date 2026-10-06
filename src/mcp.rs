// mcp.rs — MCP server: lets an AI agent drive the running engine
//
// Serves MCP over HTTP on 127.0.0.1 only. Each tool sends a `Request` to the
// engine loop, which answers it on its next tick, after the GUI's commands.
// Commands go through the same queue as the GUI's, so whatever an agent does
// shows in the window, and console commands are echoed in the Lua console.

use std::collections::VecDeque;
use std::time::Duration;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{Implementation, ServerCapabilities, ServerConfig};
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use rmcp::{tool, tool_handler, tool_router, ServerHandler};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};
use tracing::{info, warn};

use crate::gui::{EngineCommand, SimulatorSettings};
use crate::lua_interface::ScriptExecState;
use crate::world::World;

pub const DEFAULT_PORT: u16 = 7878;

/// Lua console lines kept for `get_log`.
const LOG_CAPACITY: usize = 2000;

/// Longest `wait`, s.
const MAX_WAIT: f64 = 60.0;

const INSTRUCTIONS: &str = "\
Controls a running Sysmic RoboCup SSL engine, as its user would from the GUI. \
Units: meters, radians; team 0/\"blue\", 1/\"yellow\". \
Start with get_state. The console tool runs Lua console commands: `sim help` lists \
the simulator ones (add/remove robots, robot and ball parameters, reset, scenarios). \
Lua `print` output and errors are in get_log.";

// --- Engine side --------------------------------------------------------

/// What a tool asks the engine loop for.
pub enum Request {
    State(oneshot::Sender<Snapshot>),
    /// A GUI command; answered with the state once it has been applied.
    Command(EngineCommand, oneshot::Sender<Snapshot>),
    /// A Lua console line; answered with its output.
    Console(String, oneshot::Sender<Vec<String>>),
    Log { since: u64, reply: oneshot::Sender<LogPage> },
}

/// Everything an agent sees with `get_state`.
#[derive(Debug, Serialize)]
pub struct Snapshot {
    pub simulator: SimulatorSnapshot,
    pub script: ScriptSnapshot,
    pub ball: BallSnapshot,
    pub robots: Vec<RobotSnapshot>,
}

#[derive(Debug, Serialize)]
pub struct SimulatorSnapshot {
    pub enabled: bool,
    /// Simulated seconds per real second; 0 = as fast as possible.
    pub speed: f64,
    /// Simulated time, s (only with the simulator on).
    pub time: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct ScriptSnapshot {
    /// "no_script", "running", "paused" or "failed".
    pub status: &'static str,
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct BallSnapshot {
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
}

#[derive(Debug, Serialize)]
pub struct RobotSnapshot {
    pub team: &'static str,
    pub id: i32,
    pub x: f64,
    pub y: f64,
    pub theta: f64,
    pub vx: f64,
    pub vy: f64,
    pub omega: f64,
}

impl Snapshot {
    pub fn new(
        world: &World,
        simulator: SimulatorSettings,
        sim_time: Option<f64>,
        script: ScriptExecState,
        script_path: &str,
    ) -> Self {
        let ball = world.get_ball_state();
        let robots = world
            .get_blue_team_state()
            .into_iter()
            .chain(world.get_yellow_team_state())
            .filter(|robot| robot.active)
            .map(|robot| RobotSnapshot {
                team: if robot.team == 0 { "blue" } else { "yellow" },
                id: robot.id,
                x: robot.position.x,
                y: robot.position.y,
                theta: robot.orientation,
                vx: robot.velocity.x,
                vy: robot.velocity.y,
                omega: robot.angular_velocity,
            });
        let mut robots: Vec<RobotSnapshot> = robots.collect();
        robots.sort_by_key(|robot| (robot.team, robot.id));

        Self {
            simulator: SimulatorSnapshot { enabled: simulator.enabled, speed: simulator.speed, time: sim_time },
            script: ScriptSnapshot { status: script_status(script), path: script_path.to_string() },
            ball: BallSnapshot { x: ball.position.x, y: ball.position.y, vx: ball.velocity.x, vy: ball.velocity.y },
            robots,
        }
    }
}

fn script_status(state: ScriptExecState) -> &'static str {
    match state {
        ScriptExecState::NoScript => "no_script",
        ScriptExecState::Running => "running",
        ScriptExecState::Paused => "paused",
        ScriptExecState::Failed => "failed",
    }
}

/// The Lua console's recent lines, numbered so an agent can ask for the new
/// ones only.
#[derive(Debug, Default)]
pub struct LogBuffer {
    lines: VecDeque<String>,
    /// Number of the line after the last one.
    next: u64,
}

#[derive(Debug, Serialize)]
pub struct LogPage {
    pub lines: Vec<String>,
    /// Pass as `since` to get only the lines after these.
    pub next: u64,
}

impl LogBuffer {
    pub fn push(&mut self, line: String) {
        if self.lines.len() == LOG_CAPACITY {
            self.lines.pop_front();
        }
        self.lines.push_back(line);
        self.next += 1;
    }

    /// The kept lines numbered `since` and later.
    pub fn since(&self, since: u64) -> LogPage {
        let first = self.next - self.lines.len() as u64;
        let skip = since.saturating_sub(first) as usize;
        LogPage { lines: self.lines.iter().skip(skip).cloned().collect(), next: self.next }
    }
}

// --- Server -------------------------------------------------------------

/// Serves the tools on `127.0.0.1:port` until the process ends.
pub async fn serve(port: u16, requests: mpsc::Sender<Request>) {
    let config = StreamableHttpServerConfig::default()
        .with_json_response(true)
        // Agents send no Origin; a browser page always does, so refuse those.
        .enforce_origin_validation();
    let service: StreamableHttpService<EngineTools, LocalSessionManager> =
        StreamableHttpService::new(move || Ok(EngineTools::new(requests.clone())), Default::default(), config);
    let router = axum::Router::new().nest_service("/mcp", service);

    let listener = match tokio::net::TcpListener::bind(("127.0.0.1", port)).await {
        Ok(listener) => listener,
        Err(e) => {
            warn!("MCP server not started: cannot listen on 127.0.0.1:{port}: {e}");
            return;
        }
    };
    info!("MCP server on http://127.0.0.1:{port}/mcp");
    if let Err(e) = axum::serve(listener, router).await {
        warn!("MCP server stopped: {e}");
    }
}

#[derive(Clone)]
pub struct EngineTools {
    requests: mpsc::Sender<Request>,
    #[allow(dead_code, reason = "read by the tool_handler macro")]
    tool_router: ToolRouter<Self>,
}

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum TeamName {
    Blue,
    Yellow,
}

impl TeamName {
    fn number(self) -> i32 {
        match self {
            TeamName::Blue => 0,
            TeamName::Yellow => 1,
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ConsoleArgs {
    /// A Lua console line, e.g. `sim status`, `sim add yellow 2 1.0 0.5`, `sim scenario load kickoff`.
    pub command: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TeleportRobotArgs {
    pub team: TeamName,
    pub id: u32,
    /// m
    pub x: f64,
    /// m
    pub y: f64,
    /// Heading, rad; 0 faces +x. Default 0.
    #[serde(default)]
    pub theta: f64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TeleportBallArgs {
    /// m
    pub x: f64,
    /// m
    pub y: f64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LoadScriptArgs {
    /// Path of the Lua entry script, relative to the engine's working directory
    /// (the repository root), e.g. `lua/run_ai.lua`. It loads paused.
    pub path: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SimulatorArgs {
    /// Turn the built-in simulator on (true) or go back to grSim/real robots (false).
    pub enabled: Option<bool>,
    /// Simulated seconds per real second; 0 = as fast as possible.
    pub speed: Option<f64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LogArgs {
    /// Return only lines numbered `since` and later: pass the `next` of the
    /// previous call. Omit for all kept lines.
    pub since: Option<u64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WaitArgs {
    /// Real seconds to wait, up to 60. With the simulator, simulated time
    /// advances by seconds × speed.
    pub seconds: f64,
}

#[tool_router]
impl EngineTools {
    fn new(requests: mpsc::Sender<Request>) -> Self {
        Self { requests, tool_router: Self::tool_router() }
    }

    #[tool(description = "Current state: simulator (on/off, speed, time), Lua script (status, path), ball, and every visible robot.")]
    async fn get_state(&self) -> Result<String, String> {
        let snapshot = self.ask(Request::State).await?;
        to_json(&snapshot)
    }

    #[tool(description = "Runs a line in the engine's Lua console, as if typed there, and returns its output. `sim help` lists the simulator commands: status, add, remove, reset, config, robot/ball parameters, save, scenario save/load/list.")]
    async fn console(&self, Parameters(args): Parameters<ConsoleArgs>) -> Result<String, String> {
        let reply = self.ask(|reply| Request::Console(args.command, reply)).await?.join("\n");
        // The console reports a failed command as a single `error: ...` line.
        if reply.starts_with("error: ") {
            Err(reply)
        } else {
            Ok(reply)
        }
    }

    #[tool(description = "Places a robot at rest (adds it if it is not on the field yet). Returns the new state.")]
    async fn teleport_robot(&self, Parameters(args): Parameters<TeleportRobotArgs>) -> Result<String, String> {
        let id = i32::try_from(args.id).map_err(|_| format!("robot id {} is too large", args.id))?;
        self.command(EngineCommand::TeleportRobot {
            id,
            team: args.team.number(),
            x: args.x,
            y: args.y,
            orientation: args.theta,
        })
        .await
    }

    #[tool(description = "Places the ball at rest. Returns the new state.")]
    async fn teleport_ball(&self, Parameters(args): Parameters<TeleportBallArgs>) -> Result<String, String> {
        self.command(EngineCommand::TeleportBall { x: args.x, y: args.y }).await
    }

    #[tool(description = "Loads a Lua entry script (paused; call play_script to run it). Returns the new state; check script.status and get_log for errors.")]
    async fn load_script(&self, Parameters(args): Parameters<LoadScriptArgs>) -> Result<String, String> {
        self.command(EngineCommand::LoadScript { path: args.path }).await
    }

    #[tool(description = "Runs the loaded Lua script (the GUI's play button).")]
    async fn play_script(&self) -> Result<String, String> {
        self.command(EngineCommand::ResumeScript).await
    }

    #[tool(description = "Pauses the Lua script; the robots stop.")]
    async fn pause_script(&self) -> Result<String, String> {
        self.command(EngineCommand::PauseScript).await
    }

    #[tool(description = "Turns the built-in simulator on or off and sets its speed (the SIM button and speed menu). Omitted fields keep their value.")]
    async fn set_simulator(&self, Parameters(args): Parameters<SimulatorArgs>) -> Result<String, String> {
        if args.speed.is_some_and(|speed| !(speed >= 0.0)) {
            return Err("speed must be >= 0 (0 = as fast as possible)".to_string());
        }
        let current = self.ask(Request::State).await?.simulator;
        let settings = SimulatorSettings {
            enabled: args.enabled.unwrap_or(current.enabled),
            speed: args.speed.unwrap_or(current.speed),
        };
        self.command(EngineCommand::SetSimulator(settings)).await
    }

    #[tool(description = "Lua console output (print, errors, console replies), oldest first, with `next` to pass as `since` next time.")]
    async fn get_log(&self, Parameters(args): Parameters<LogArgs>) -> Result<String, String> {
        let since = args.since.unwrap_or(0);
        let page = self.ask(|reply| Request::Log { since, reply }).await?;
        to_json(&page)
    }

    #[tool(description = "Lets the engine run for some real seconds, then returns the state. Use it to watch what a running script does.")]
    async fn wait(&self, Parameters(args): Parameters<WaitArgs>) -> Result<String, String> {
        if !(args.seconds >= 0.0 && args.seconds <= MAX_WAIT) {
            return Err(format!("seconds must be between 0 and {MAX_WAIT}"));
        }
        tokio::time::sleep(Duration::from_secs_f64(args.seconds)).await;
        self.get_state().await
    }
}

impl EngineTools {
    /// Sends a request to the engine loop and waits for its answer.
    async fn ask<T>(&self, request: impl FnOnce(oneshot::Sender<T>) -> Request) -> Result<T, String> {
        let (reply, answer) = oneshot::channel();
        self.requests.send(request(reply)).await.map_err(|_| "the engine has stopped".to_string())?;
        answer.await.map_err(|_| "the engine dropped the request".to_string())
    }

    async fn command(&self, command: EngineCommand) -> Result<String, String> {
        let snapshot = self.ask(|reply| Request::Command(command, reply)).await?;
        to_json(&snapshot)
    }
}

#[tool_handler]
impl ServerHandler for EngineTools {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("sysmic-engine", env!("CARGO_PKG_VERSION")))
            .with_instructions(INSTRUCTIONS)
    }
}

fn to_json(value: &impl Serialize) -> Result<String, String> {
    serde_json::to_string_pretty(value).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_pages_return_only_new_lines() {
        let mut log = LogBuffer::default();
        log.push("a".into());
        log.push("b".into());
        let page = log.since(0);
        assert_eq!(page.lines, ["a", "b"]);

        log.push("c".into());
        assert_eq!(log.since(page.next).lines, ["c"]);
        assert!(log.since(log.since(0).next).lines.is_empty());
    }

    #[test]
    fn log_keeps_the_latest_lines() {
        let mut log = LogBuffer::default();
        for i in 0..LOG_CAPACITY + 5 {
            log.push(i.to_string());
        }
        let page = log.since(0);
        assert_eq!(page.lines.len(), LOG_CAPACITY);
        assert_eq!(page.lines[0], "5");
        assert_eq!(page.next, (LOG_CAPACITY + 5) as u64);
    }
}
