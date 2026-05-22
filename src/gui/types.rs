use iced::event::Event;
use iced::window;
use tokio::sync::mpsc;

use crate::types::Vec2D;

use super::bottom_panel::BottomPanelMessage;
use super::panels::charts::ChartsMessage;
use super::panels::control::ControlMessage;
use super::panels::kalman::KalmanMessage;
use super::panels::radio::RadioMessage;
use super::panels::recording::RecordingMessage;
use super::panels::vision::VisionMessage;
use super::sidebar::SidebarMessage;
use super::toolbar::{self, ToolbarMessage};

// --- Vision update (sent from vision task to GUI) ---
#[derive(Debug, Clone)]
pub struct VisionUpdate {
    pub ball: Option<Vec2D>,
    pub robots_blue: Vec<RobotUpdateData>,
    pub robots_yellow: Vec<RobotUpdateData>,
    pub pps: u32,
}

#[derive(Debug, Clone)]
pub struct RobotUpdateData {
    pub id: u32,
    pub x: f64,
    pub y: f64,
    pub theta: f64,
    pub vx: f64,
    pub vy: f64,
    pub cmd_vx: f64,
    pub cmd_vy: f64,
    pub cmd_angular: f64,
}

// --- Lua draw commands (sent from engine to GUI) ---
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum LuaDrawCmd {
    Point {
        x: f64,
        y: f64,
        draw_x: bool,
        color: Option<[f32; 3]>,
    },
    HighlightRobot { id: i32, team: i32 },
    Line {
        points: Vec<(f64, f64)>,
        draw_points_between: bool,
        color: Option<[f32; 3]>,
    },
    Text {
        x: f64,
        y: f64,
        text: String,
        color: Option<[f32; 3]>,
    },
}

// --- Commands from GUI to engine ---
#[derive(Debug, Clone)]
pub enum EngineCommand {
    UpdateVisionConnection { ip: String, port: u16 },
    UpdateRadioConfig { use_radio: bool, port_name: String, baud_rate: u32 },
    UpdateTrackerConfig { enabled: bool, process_noise_p: f64, process_noise_v: f64, measurement_noise: f64 },
    StartRecording { filename: String },
    StopRecording,
    SendRobotCommand { id: i32, team: i32, vx: f64, vy: f64, omega: f64 },
    SendKickCommand { id: i32, team: i32 },
    LoadScript { path: String },
    PauseScript,
    ResumeScript,
}

// --- GUI channels ---
pub struct GuiChannels {
    pub vision_rx: mpsc::Receiver<VisionUpdate>,
    pub lua_draw_rx: mpsc::Receiver<Vec<LuaDrawCmd>>,
    pub lua_status_rx: mpsc::Receiver<LuaScriptStatusUpdate>,
    pub lua_log_rx: mpsc::Receiver<String>,
    pub command_tx: mpsc::Sender<EngineCommand>,
}

#[derive(Debug, Clone)]
pub struct LuaScriptStatusUpdate {
    pub status: toolbar::ScriptStatus,
    pub script_path: Option<String>,
}

// --- Main Application Message ---
#[derive(Debug, Clone)]
pub enum Message {
    // Sub-component messages
    Sidebar(SidebarMessage),
    Toolbar(ToolbarMessage),
    BottomPanel(BottomPanelMessage),
    Vision(VisionMessage),
    Radio(RadioMessage),
    Kalman(KalmanMessage),
    Recording(RecordingMessage),
    Control(ControlMessage),
    Charts(ChartsMessage),

    // Events
    Tick,
    EventOccurred(Event),
    ScriptFileSelected(Option<String>),
    ReplayFilePick,
    ReplayFileSelected(Option<String>),
    ReplayPlay,
    ReplayPause,
    ReplaySeek(u32),
    LuaConsoleResizeStart,

    // Window events
    WindowOpened(window::Id),
    WindowClosed(window::Id),
}
