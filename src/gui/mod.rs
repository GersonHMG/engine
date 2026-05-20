// gui/mod.rs — Main Iced daemon application module for the Sysmic Engine GUI

pub mod field_canvas;
pub mod sidebar;
pub mod toolbar;
pub mod bottom_panel;
pub mod panels;
pub mod lua_console;

use iced::widget::{button, column, container, row, scrollable, slider, text};
use iced::widget::operation::snap_to_end;
use iced::{Element, Length, Subscription, Theme};
use iced::event::{self, Event};
use iced::keyboard;
use iced::mouse;
use iced::window;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tracing::warn;

use crate::types::Vec2D;
use crate::config::FieldConfig;

use field_canvas::{FieldCanvas, FieldData, LuaDrawCommand, RobotData};
use sidebar::{Sidebar, SidebarMessage, SidebarPanel};
use toolbar::{Toolbar, ToolbarMessage};
use bottom_panel::{BottomPanel, BottomPanelMessage};
use lua_console::LuaConsolePanel;
use panels::vision::{VisionPanel, VisionMessage};
use panels::radio::{RadioPanel, RadioMessage};
use panels::kalman::{KalmanPanel, KalmanMessage};
use panels::recording::{RecordingPanel, RecordingMessage, RecordingStatus};
use panels::control::{ControlPanel, ControlMessage};
use panels::charts::{ChartsPanel, ChartsMessage, DataSample, export_csv_async};

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

#[derive(Debug, Clone)]
struct ReplayFrame {
    elapsed_ms: u64,
    robots_blue: Vec<RobotData>,
    robots_yellow: Vec<RobotData>,
    ball: (f64, f64),
}

#[derive(Default)]
struct ReplayFrameBuilder {
    robots_blue: HashMap<u32, RobotData>,
    robots_yellow: HashMap<u32, RobotData>,
    ball: (f64, f64),
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

// --- Main Application ---
pub struct EngineApp {
    // Sub-components
    field_canvas: FieldCanvas,
    sidebar: Sidebar,
    toolbar: Toolbar,
    bottom_panel: BottomPanel,

    // Panels
    vision_panel: VisionPanel,
    radio_panel: RadioPanel,
    kalman_panel: KalmanPanel,
    recording_panel: RecordingPanel,
    control_panel: ControlPanel,
    charts_panel: ChartsPanel,

    // Data
    field_data: FieldData,
    robot_trace: Vec<(f64, f64)>,
    last_vision_time: std::time::Instant,
    last_pps_time: std::time::Instant,

    // Channels (wrapped in Arc<Mutex> for Iced)
    vision_rx: Arc<Mutex<mpsc::Receiver<VisionUpdate>>>,
    lua_draw_rx: Arc<Mutex<mpsc::Receiver<Vec<LuaDrawCmd>>>>,
    lua_status_rx: Arc<Mutex<mpsc::Receiver<LuaScriptStatusUpdate>>>,
    lua_log_rx: Arc<Mutex<mpsc::Receiver<String>>>,
    command_tx: mpsc::Sender<EngineCommand>,

    // Keyboard state for manual control
    key_chars: std::collections::HashSet<char>,
    kick_key_was_down: bool,

    // Mouse state
    last_cursor_position: Option<iced::Point>,

    // Window management
    main_window_id: window::Id,
    panel_windows: HashMap<SidebarPanel, window::Id>,
    window_to_panel: HashMap<window::Id, SidebarPanel>,

    // Lua console panel
    lua_console_panel: LuaConsolePanel,
    lua_console_resizing: bool,
    lua_console_resize_start_y: f32,
    lua_console_resize_start_height: f32,
    window_height: f32,

    // Replay mode UI state
    replay_mode: bool,
    replay_file_path: Option<String>,
    replay_is_playing: bool,
    replay_frames: Vec<ReplayFrame>,
    replay_frame_index: usize,
    replay_elapsed_ms: u64,
    replay_last_tick: std::time::Instant,
}

impl EngineApp {
    /// Boot function for iced::daemon — opens the main window and returns initial state + tasks
    pub fn boot(channels: GuiChannels, field_config: FieldConfig) -> (Self, iced::Task<Message>) {
        let (main_id, open_task) = window::open(window::Settings {
            size: iced::Size::new(900.0, 600.0),
            ..Default::default()
        });

        let app = Self {
            field_canvas: FieldCanvas::new(),
            sidebar: Sidebar::new(),
            toolbar: Toolbar::new(),
            bottom_panel: BottomPanel::new(),
            vision_panel: VisionPanel::default(),
            radio_panel: RadioPanel::default(),
            kalman_panel: KalmanPanel::default(),
            recording_panel: RecordingPanel::default(),
            control_panel: ControlPanel::default(),
            charts_panel: ChartsPanel::new(),
            field_data: FieldData::new(field_config.length_m, field_config.width_m),
            robot_trace: Vec::new(),
            last_vision_time: std::time::Instant::now(),
            last_pps_time: std::time::Instant::now(),
            vision_rx: Arc::new(Mutex::new(channels.vision_rx)),
            lua_draw_rx: Arc::new(Mutex::new(channels.lua_draw_rx)),
            lua_status_rx: Arc::new(Mutex::new(channels.lua_status_rx)),
            lua_log_rx: Arc::new(Mutex::new(channels.lua_log_rx)),
            command_tx: channels.command_tx,
            key_chars: std::collections::HashSet::new(),
            kick_key_was_down: false,
            last_cursor_position: None,
            main_window_id: main_id,
            panel_windows: HashMap::new(),
            window_to_panel: HashMap::new(),
            lua_console_panel: LuaConsolePanel::new(),
            lua_console_resizing: false,
            lua_console_resize_start_y: 0.0,
            lua_console_resize_start_height: 0.0,
            window_height: 800.0,
            replay_mode: false,
            replay_file_path: None,
            replay_is_playing: false,
            replay_frames: Vec::new(),
            replay_frame_index: 0,
            replay_elapsed_ms: 0,
            replay_last_tick: std::time::Instant::now(),
        };

        (app, open_task.map(Message::WindowOpened))
    }

    pub fn title(&self, window_id: window::Id) -> String {
        if window_id == self.main_window_id {
            "Sysmic Engine".to_string()
        } else if let Some(panel) = self.window_to_panel.get(&window_id) {
            match panel {
                SidebarPanel::Vision => "Vision Settings".to_string(),
                SidebarPanel::Radio => "Radio Settings".to_string(),
                SidebarPanel::Kalman => "Kalman Filter".to_string(),
                SidebarPanel::Recording => "Recording".to_string(),
                SidebarPanel::Control => "Manual Control".to_string(),
                SidebarPanel::Charts => "Robot Charts".to_string(),
                SidebarPanel::LuaConsole => "Lua Console".to_string(),
            }
        } else {
            "Sysmic Engine".to_string()
        }
    }

    pub fn theme(&self, _window_id: window::Id) -> Theme {
        Theme::Dark
    }

    fn open_panel_window(&mut self, panel: SidebarPanel) -> iced::Task<Message> {
        if self.panel_windows.contains_key(&panel) {
            // Bring existing window to front by focusing it
            if let Some(&wid) = self.panel_windows.get(&panel) {
                return window::gain_focus(wid);
            }
            return iced::Task::none();
        }

        let size = if panel == SidebarPanel::Charts {
            iced::Size::new(640.0, 400.0)
        } else {
            iced::Size::new(300.0, 420.0)
        };

        let (id, task) = window::open(window::Settings {
            size,
            resizable: true,
            ..Default::default()
        });

        self.panel_windows.insert(panel, id);
        self.window_to_panel.insert(id, panel);
        self.sidebar.active_panel = Some(panel);

        task.map(Message::WindowOpened)
    }

    fn close_panel_window(&mut self, panel: SidebarPanel) -> iced::Task<Message> {
        if let Some(id) = self.panel_windows.remove(&panel) {
            self.window_to_panel.remove(&id);
            if self.sidebar.active_panel == Some(panel) {
                self.sidebar.active_panel = None;
            }
            return window::close(id);
        }
        iced::Task::none()
    }

    pub fn update(&mut self, message: Message) -> iced::Task<Message> {
        if self.replay_mode {
            match &message {
                Message::Sidebar(SidebarMessage::ToggleReplayMode)
                | Message::ReplayFilePick
                | Message::ReplayFileSelected(_)
                | Message::ReplayPlay
                | Message::ReplayPause
                | Message::ReplaySeek(_)
                | Message::Tick
                | Message::EventOccurred(_)
                | Message::WindowOpened(_)
                | Message::WindowClosed(_) => {}
                _ => return iced::Task::none(),
            }
        }

        match message {
            // --- Sidebar ---
            Message::Sidebar(SidebarMessage::ToggleReplayMode) => {
                self.replay_mode = !self.replay_mode;

                if self.replay_mode {
                    let panel_ids: Vec<window::Id> = self.panel_windows.values().cloned().collect();
                    self.panel_windows.clear();
                    self.window_to_panel.clear();
                    self.sidebar.active_panel = None;
                    self.lua_console_panel.open = false;
                    self.key_chars.clear();
                    self.kick_key_was_down = false;
                    self.replay_is_playing = false;
                    self.replay_last_tick = std::time::Instant::now();

                    let tasks: Vec<iced::Task<Message>> = panel_ids
                        .into_iter()
                        .map(window::close)
                        .collect();
                    return iced::Task::batch(tasks);
                }

                self.replay_is_playing = false;
            }
            Message::Sidebar(SidebarMessage::TogglePanel(panel)) => {
                if self.replay_mode {
                    return iced::Task::none();
                }
                if panel == SidebarPanel::LuaConsole {
                    self.lua_console_panel.open = !self.lua_console_panel.open;
                    return iced::Task::none();
                }
                if self.panel_windows.contains_key(&panel) {
                    return self.close_panel_window(panel);
                } else {
                    return self.open_panel_window(panel);
                }
            }

            Message::LuaConsoleResizeStart => {
                if let Some(pos) = self.last_cursor_position {
                    self.lua_console_resizing = true;
                    self.lua_console_resize_start_y = pos.y;
                    self.lua_console_resize_start_height = self.lua_console_panel.height();
                }
            }

            Message::ReplayFilePick => {
                return iced::Task::perform(
                    async {
                        let result = rfd::AsyncFileDialog::new()
                            .add_filter("CSV Files", &["csv"])
                            .pick_file()
                            .await;
                        result.map(|f| f.path().to_string_lossy().to_string())
                    },
                    Message::ReplayFileSelected,
                );
            }
            Message::ReplayFileSelected(Some(path)) => {
                self.replay_file_path = Some(path);
                self.replay_is_playing = false;
                self.replay_last_tick = std::time::Instant::now();

                match Self::load_replay_frames(self.replay_file_path.as_deref().unwrap_or_default()) {
                    Ok(frames) => {
                        self.replay_frames = frames;
                        self.replay_frame_index = 0;
                        self.replay_elapsed_ms = self.replay_frames.first().map(|f| f.elapsed_ms).unwrap_or(0);
                        self.apply_replay_frame();
                    }
                    Err(e) => {
                        self.replay_frames.clear();
                        self.replay_frame_index = 0;
                        self.replay_elapsed_ms = 0;
                        warn!("Replay: failed to load CSV: {e}");
                    }
                }
            }
            Message::ReplayFileSelected(None) => {}
            Message::ReplayPlay => {
                if !self.replay_frames.is_empty() {
                    if self.replay_frame_index + 1 >= self.replay_frames.len() {
                        self.replay_frame_index = 0;
                        self.replay_elapsed_ms = self.replay_frames[0].elapsed_ms;
                        self.apply_replay_frame();
                    }
                    self.replay_is_playing = true;
                    self.replay_last_tick = std::time::Instant::now();
                }
            }
            Message::ReplayPause => {
                self.replay_is_playing = false;
            }
            Message::ReplaySeek(index) => {
                if self.replay_frames.is_empty() {
                    return iced::Task::none();
                }

                self.replay_is_playing = false;
                self.replay_last_tick = std::time::Instant::now();

                let idx = (index as usize).min(self.replay_frames.len().saturating_sub(1));
                self.replay_frame_index = idx;
                self.replay_elapsed_ms = self.replay_frames[idx].elapsed_ms;
                self.apply_replay_frame();
            }

            // --- Toolbar ---
            Message::Toolbar(ToolbarMessage::LoadScript) => {
                return iced::Task::perform(
                    async {
                        let result = rfd::AsyncFileDialog::new()
                            .add_filter("Lua Scripts", &["lua"])
                            .pick_file()
                            .await;
                        result.map(|f| f.path().to_string_lossy().to_string())
                    },
                    Message::ScriptFileSelected,
                );
            }
            Message::Toolbar(ToolbarMessage::ToggleScript) => {
                if self.toolbar.script_status == toolbar::ScriptStatus::Running {
                    let _ = self.command_tx.try_send(EngineCommand::PauseScript);
                } else {
                    if self.toolbar.script_status != toolbar::ScriptStatus::NoScript {
                        let _ = self.command_tx.try_send(EngineCommand::ResumeScript);
                    }
                }
            }
            Message::Toolbar(ToolbarMessage::ReloadScript) => {
                if !self.toolbar.script_path.is_empty() {
                    let path = self.toolbar.script_path.clone();
                    let _ = self.command_tx.try_send(EngineCommand::LoadScript { path });
                }
            }
            Message::ScriptFileSelected(Some(path)) => {
                let _ = self.command_tx.try_send(EngineCommand::LoadScript { path: path.clone() });
                self.toolbar.script_path = path;
            }
            Message::ScriptFileSelected(None) => {}

            // --- Bottom Panel ---
            Message::BottomPanel(BottomPanelMessage::SetTrace(val)) => {
                self.bottom_panel.trace_on = val;
                self.field_data.robot_trace.clear();
                self.robot_trace.clear();
            }
            Message::BottomPanel(BottomPanelMessage::SetVectors(val)) => {
                self.bottom_panel.vectors_on = val;
                self.field_data.vis_velocities = val;
            }
            Message::BottomPanel(BottomPanelMessage::SetHighlight(val)) => {
                self.bottom_panel.highlight_on = val;
                if val {
                    let id = self.bottom_panel.control_robot_id.parse::<u32>().unwrap_or(0);
                    let team = self.bottom_panel.control_team.to_id();
                    self.field_data.highlight_robot = Some((id, team));
                } else {
                    self.field_data.highlight_robot = None;
                }
            }
            Message::BottomPanel(BottomPanelMessage::SetManualControl(val)) => {
                self.bottom_panel.manual_control_on = val;
                self.control_panel.active = val;
            }
            Message::BottomPanel(BottomPanelMessage::IncrementRobotId) => {
                let new_id = (self.bottom_panel.control_robot_id.parse::<i32>().unwrap_or(0) + 1).min(12);
                let s = new_id.to_string();
                self.bottom_panel.control_robot_id = s.clone();
                self.control_panel.robot_id = s.clone();
                if self.bottom_panel.highlight_on {
                    self.field_data.highlight_robot = Some((new_id as u32, self.bottom_panel.control_team.to_id()));
                }
            }
            Message::BottomPanel(BottomPanelMessage::DecrementRobotId) => {
                let new_id = (self.bottom_panel.control_robot_id.parse::<i32>().unwrap_or(0) - 1).max(0);
                let s = new_id.to_string();
                self.bottom_panel.control_robot_id = s.clone();
                self.control_panel.robot_id = s.clone();
                if self.bottom_panel.highlight_on {
                    self.field_data.highlight_robot = Some((new_id as u32, self.bottom_panel.control_team.to_id()));
                }
            }
            Message::BottomPanel(BottomPanelMessage::TeamSelected(team)) => {
                self.bottom_panel.control_team = team;
                self.control_panel.team = team;
                if self.bottom_panel.highlight_on {
                    let id = self.bottom_panel.control_robot_id.parse::<u32>().unwrap_or(0);
                    self.field_data.highlight_robot = Some((id, team.to_id()));
                }
            }
            Message::BottomPanel(BottomPanelMessage::RobotIdChanged(id)) => {
                // Accept only digits, clamp to 0-12
                let filtered: String = id.chars().filter(|c| c.is_ascii_digit()).collect();
                let clamped = filtered.parse::<i32>().unwrap_or(0).clamp(0, 12).to_string();
                // Keep raw input while typing so user can clear and retype; only clamp when valid
                let new_id = if filtered.is_empty() { filtered.clone() } else { clamped };
                self.bottom_panel.control_robot_id = new_id.clone();
                self.control_panel.robot_id = new_id.clone();
                if self.bottom_panel.highlight_on {
                    let robot_id = new_id.parse::<u32>().unwrap_or(0);
                    let team = self.bottom_panel.control_team.to_id();
                    self.field_data.highlight_robot = Some((robot_id, team));
                }
            }

            // --- Vision Panel ---
            Message::Vision(VisionMessage::IpChanged(ip)) => {
                self.vision_panel.ip = ip;
            }
            Message::Vision(VisionMessage::PortChanged(port)) => {
                self.vision_panel.port = port;
            }
            Message::Vision(VisionMessage::Reconnect) => {
                if let Ok(port) = self.vision_panel.port.parse::<u16>() {
                    let _ = self.command_tx.try_send(EngineCommand::UpdateVisionConnection {
                        ip: self.vision_panel.ip.clone(),
                        port,
                    });
                }
            }

            // --- Radio Panel ---
            Message::Radio(RadioMessage::PortNameChanged(name)) => {
                self.radio_panel.port_name = name;
            }
            Message::Radio(RadioMessage::BaudRateChanged(rate)) => {
                self.radio_panel.baud_rate = rate;
            }
            Message::Radio(RadioMessage::UseRadioToggled(val)) => {
                self.radio_panel.use_radio = val;
            }
            Message::Radio(RadioMessage::Update) => {
                let baud = self.radio_panel.baud_rate.parse::<u32>().unwrap_or(115200);
                let _ = self.command_tx.try_send(EngineCommand::UpdateRadioConfig {
                    use_radio: self.radio_panel.use_radio,
                    port_name: self.radio_panel.port_name.clone(),
                    baud_rate: baud,
                });
            }

            // --- Kalman Panel ---
            Message::Kalman(KalmanMessage::EnabledToggled(val)) => {
                self.kalman_panel.enabled = val;
            }
            Message::Kalman(KalmanMessage::ProcessNoisePChanged(val)) => {
                self.kalman_panel.process_noise_p = val;
            }
            Message::Kalman(KalmanMessage::ProcessNoiseVChanged(val)) => {
                self.kalman_panel.process_noise_v = val;
            }
            Message::Kalman(KalmanMessage::MeasurementNoiseChanged(val)) => {
                self.kalman_panel.measurement_noise = val;
            }
            Message::Kalman(KalmanMessage::Update) => {
                let p = self.kalman_panel.process_noise_p.parse::<f64>().unwrap_or(0.0000001);
                let v = self.kalman_panel.process_noise_v.parse::<f64>().unwrap_or(0.0001);
                let m = self.kalman_panel.measurement_noise.parse::<f64>().unwrap_or(0.000001);
                let _ = self.command_tx.try_send(EngineCommand::UpdateTrackerConfig {
                    enabled: self.kalman_panel.enabled,
                    process_noise_p: p,
                    process_noise_v: v,
                    measurement_noise: m,
                });
            }

            // --- Recording Panel ---
            Message::Recording(RecordingMessage::FilenameChanged(name)) => {
                self.recording_panel.filename = name;
            }
            Message::Recording(RecordingMessage::Start) => {
                let _ = self.command_tx.try_send(EngineCommand::StartRecording {
                    filename: self.recording_panel.filename.clone(),
                });
                self.recording_panel.status = RecordingStatus::Recording;
            }
            Message::Recording(RecordingMessage::Stop) => {
                let _ = self.command_tx.try_send(EngineCommand::StopRecording);
                self.recording_panel.status = RecordingStatus::Saved;
            }

            // --- Charts Panel ---
            Message::Charts(ref msg) => {
                let needs_export = self.charts_panel.update(msg);
                if needs_export {
                    let samples = self.charts_panel.recorded_samples_clone();
                    return iced::Task::perform(
                        async move { export_csv_async(samples).await },
                        |_| Message::Charts(ChartsMessage::ExportComplete),
                    );
                }
            }

            // --- Control Panel ---
            Message::Control(ControlMessage::ModeSelected(mode)) => {
                self.control_panel.mode = mode;
            }
            Message::Control(ControlMessage::ScaleVxChanged(val)) => {
                self.control_panel.scale_vx = val;
            }
            Message::Control(ControlMessage::ScaleVyChanged(val)) => {
                self.control_panel.scale_vy = val;
            }
            Message::Control(ControlMessage::ScaleWChanged(val)) => {
                self.control_panel.scale_w = val;
            }

            // --- Tick (periodic update) ---
            Message::Tick => {
                if self.replay_mode {
                    self.tick_replay();
                    self.field_canvas.request_redraw();
                    return iced::Task::none();
                }

                let elapsed = self.last_vision_time.elapsed();
                self.field_data.vision_connected = elapsed.as_millis() < 1000;
                if !self.field_data.vision_connected {
                    self.vision_panel.connected = false;
                    self.vision_panel.pps = 0;
                    self.toolbar.pps = 0;
                }

                self.poll_keyboard_control();

                // Throttle PPS sparkline pushes to every 250ms (~4Hz instead of 60Hz)
                let mut spark_push = false;
                if self.last_pps_time.elapsed().as_millis() > 250 {
                    self.last_pps_time = std::time::Instant::now();
                    spark_push = true;
                }
                
                if spark_push && !self.field_data.vision_connected {
                    self.toolbar.push_pps(0);
                }

                // Drain vision channel
                if let Ok(mut rx) = self.vision_rx.try_lock() {
                    while let Ok(update) = rx.try_recv() {
                        self.last_vision_time = std::time::Instant::now();
                        self.vision_panel.connected = true;
                        self.vision_panel.pps = update.pps;
                        self.toolbar.pps = update.pps;
                        if spark_push {
                            self.toolbar.push_pps(update.pps);
                            spark_push = false; // Prevents pushing multiple times per batch if loop repeats
                        }

                        self.field_data.robots_blue = update.robots_blue.iter()
                            .map(|r| RobotData {
                                id: r.id, x: r.x, y: r.y, theta: r.theta,
                                vx: r.vx, vy: r.vy,
                                cmd_vx: r.cmd_vx, cmd_vy: r.cmd_vy, cmd_angular: r.cmd_angular,
                            })
                            .collect();
                        self.field_data.robots_yellow = update.robots_yellow.iter()
                            .map(|r| RobotData {
                                id: r.id, x: r.x, y: r.y, theta: r.theta,
                                vx: r.vx, vy: r.vy,
                                cmd_vx: r.cmd_vx, cmd_vy: r.cmd_vy, cmd_angular: r.cmd_angular,
                            })
                            .collect();
                        if let Some(b) = update.ball {
                            self.field_data.ball = (b.x, b.y);
                        }

                        // Charts — feed data to charts panel
                        let ctrl_id = self.control_panel.robot_id_parsed();
                        let ctrl_team = self.control_panel.team.to_id();
                        let robots = if ctrl_team == 0 { &update.robots_blue } else { &update.robots_yellow };
                        if let Some(target) = robots.iter().find(|r| r.id == ctrl_id as u32) {
                            if self.charts_panel.active {
                                let elapsed = self.charts_panel.start_time.elapsed().as_secs_f64();
                                self.charts_panel.push_sample(DataSample {
                                    time: elapsed,
                                    x: target.x,
                                    y: target.y,
                                    theta: target.theta,
                                    vx: target.vx,
                                    vy: target.vy,
                                    cmd_vx: target.cmd_vx,
                                    cmd_vy: target.cmd_vy,
                                    cmd_angular: target.cmd_angular,
                                });
                            }
                            if self.bottom_panel.trace_on {
                                self.robot_trace.push((target.x, target.y));
                                self.field_data.robot_trace = self.robot_trace.clone();
                            }
                        }
                    }
                }

                // Drain lua draw channel
                if let Ok(mut rx) = self.lua_draw_rx.try_lock() {
                    while let Ok(cmds) = rx.try_recv() {
                        self.field_data.lua_draw_commands = cmds.iter()
                            .map(|cmd| match cmd {
                                LuaDrawCmd::Point { x, y, draw_x, color } => LuaDrawCommand::Point {
                                    x: *x,
                                    y: *y,
                                    draw_x: *draw_x,
                                    color: *color,
                                },
                                LuaDrawCmd::HighlightRobot { id, team } => LuaDrawCommand::HighlightRobot { id: *id, team: *team },
                                LuaDrawCmd::Line {
                                    points,
                                    draw_points_between,
                                    color,
                                } => LuaDrawCommand::Line {
                                    points: points.clone(),
                                    draw_points_between: *draw_points_between,
                                    color: *color,
                                },
                                LuaDrawCmd::Text { x, y, text, color } => LuaDrawCommand::Text {
                                    x: *x,
                                    y: *y,
                                    text: text.clone(),
                                    color: *color,
                                },
                            })
                            .collect();
                    }
                }

                // Drain Lua script status channel
                if let Ok(mut rx) = self.lua_status_rx.try_lock() {
                    while let Ok(update) = rx.try_recv() {
                        let status = update.status.clone();
                        self.toolbar.script_status = status.clone();
                        if status == toolbar::ScriptStatus::Failed {
                            self.lua_console_panel.open = true;
                        }
                        if let Some(path) = update.script_path {
                            self.toolbar.script_path = path;
                        } else if status == toolbar::ScriptStatus::NoScript {
                            self.toolbar.script_path.clear();
                        }
                    }
                }

                // Drain Lua log channel
                let mut log_added = false;
                if let Ok(mut rx) = self.lua_log_rx.try_lock() {
                    while let Ok(line) = rx.try_recv() {
                        if self.lua_console_panel.push_line(line) {
                            log_added = true;
                        }
                    }
                }

                let mut scroll_task = iced::Task::none();
                if log_added && self.lua_console_panel.open {
                    scroll_task = snap_to_end(self.lua_console_panel.scroll_id());
                }

                self.field_canvas.request_redraw();
                return scroll_task;
            }

            // --- Global events (keyboard + mouse) ---
            Message::EventOccurred(event) => {
                match event {
                    Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                        if self.lua_console_resizing {
                            self.lua_console_resizing = false;
                            return iced::Task::none();
                        }
                        self.field_canvas.handle_drag_end();
                    }
                    Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                        if let Some(pos) = self.last_cursor_position {
                            if self.lua_console_panel.open && !self.replay_mode {
                                let bottom_height = if self.replay_mode { 0.0 } else { 52.0 };
                                let main_row_height = (self.window_height - bottom_height).max(0.0);
                                let panel_top = main_row_height - self.lua_console_panel.height();
                                if pos.y >= panel_top && pos.y <= panel_top + lua_console::RESIZE_EDGE_PX {
                                    self.lua_console_resizing = true;
                                    self.lua_console_resize_start_y = pos.y;
                                    self.lua_console_resize_start_height = self.lua_console_panel.height();
                                    return iced::Task::none();
                                }
                            }
                            self.field_canvas.handle_drag_start(pos);
                        }
                    }
                    Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) => {
                        if let keyboard::Key::Character(c) = &key {
                            let ch = c.chars().next().unwrap_or(' ').to_ascii_lowercase();
                            self.key_chars.insert(ch);
                        }
                    }
                    Event::Keyboard(keyboard::Event::KeyReleased { key, .. }) => {
                        if let keyboard::Key::Character(c) = &key {
                            let ch = c.chars().next().unwrap_or(' ').to_ascii_lowercase();
                            self.key_chars.remove(&ch);
                        }
                    }
                    Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                        let y = match delta {
                            mouse::ScrollDelta::Lines { y, .. } => y,
                            mouse::ScrollDelta::Pixels { y, .. } => y / 100.0,
                        };
                        self.field_canvas.handle_scroll(y);
                    }
                    Event::Mouse(mouse::Event::CursorMoved { position }) => {
                        self.last_cursor_position = Some(iced::Point::new(position.x, position.y));
                        if self.lua_console_resizing {
                            let delta = position.y - self.lua_console_resize_start_y;
                            let new_height = self.lua_console_resize_start_height - delta;
                            self.lua_console_panel.set_height(new_height);
                            return iced::Task::none();
                        }
                        self.field_canvas.handle_drag_move(iced::Point::new(position.x, position.y));
                        self.field_canvas.update_mouse_pos(iced::Point::new(position.x, position.y));
                    }
                    Event::Window(window::Event::Resized(size)) => {
                        self.window_height = size.height;
                    }
                    _ => {}
                }
            }


            // --- Window events ---
            Message::WindowOpened(id) => {
                if self.window_to_panel.get(&id) == Some(&SidebarPanel::Charts) {
                    self.charts_panel.active = true;
                }
            }
            Message::WindowClosed(id) => {
                if id == self.main_window_id {
                    let panel_ids: Vec<window::Id> = self.panel_windows.values().cloned().collect();
                    let mut tasks: Vec<iced::Task<Message>> = panel_ids.iter().map(|wid| window::close(*wid)).collect();
                    tasks.push(iced::exit());
                    return iced::Task::batch(tasks);
                } else {
                    if let Some(panel) = self.window_to_panel.remove(&id) {
                        if panel == SidebarPanel::Charts {
                            self.charts_panel.active = false;
                        }
                        self.panel_windows.remove(&panel);
                        if self.sidebar.active_panel == Some(panel) {
                            self.sidebar.active_panel = None;
                        }
                    }
                }
            }
        }

        iced::Task::none()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let tick = iced::time::every(std::time::Duration::from_millis(16)).map(|_| Message::Tick);
        let events = event::listen().map(Message::EventOccurred);
        let close_events = window::close_events().map(Message::WindowClosed);
        Subscription::batch([tick, events, close_events])
    }

    pub fn view(&self, window_id: window::Id) -> Element<'_, Message> {
        if window_id == self.main_window_id {
            self.view_main()
        } else if let Some(panel) = self.window_to_panel.get(&window_id) {
            self.view_panel(*panel)
        } else {
            text("Unknown window").into()
        }
    }

    fn view_main(&self) -> Element<'_, Message> {
        // Sidebar
        let sidebar = self
            .sidebar
            .view(self.replay_mode, self.lua_console_panel.open)
            .map(Message::Sidebar);

        // Toolbar
        let toolbar: Element<'_, Message> = if self.replay_mode {
            let replay_file_name = self
                .replay_file_path
                .as_ref()
                .map(|p| p.replace('\\', "/").rsplit('/').next().unwrap_or("No file").to_string())
                .unwrap_or_else(|| "No CSV selected".to_string());

            let open_btn = button(text("Open CSV").size(12))
                .on_press(Message::ReplayFilePick)
                .style(button::secondary);

            container(
                row![
                    text("Replay Mode").size(12).color(iced::Color::from_rgb(0.8, 0.8, 0.8)),
                    open_btn,
                    text(replay_file_name).size(12).color(iced::Color::from_rgb(0.6, 0.6, 0.6)),
                ]
                .spacing(8)
                .padding(4)
                .align_y(iced::Alignment::Center),
            )
            .width(Length::Fill)
            .height(Length::Fixed(36.0))
            .style(|theme: &iced::Theme| {
                let palette = theme.extended_palette();
                container::Style {
                    background: Some(iced::Background::Color(palette.background.weak.color)),
                    border: iced::Border {
                        color: palette.background.strong.color,
                        width: 1.0,
                        radius: 0.0.into(),
                    },
                    ..Default::default()
                }
            })
            .into()
        } else {
            self.toolbar.view().map(Message::Toolbar)
        };

        // Field canvas
        let canvas: Element<'_, Message> = self.field_canvas.view(&self.field_data);

        // Mouse coords overlay
        let mouse_pos_text = if let Some((x, y)) = self.field_canvas.mouse_field_pos() {
            format!("{:.2}, {:.2}", x, y)
        } else {
            "0.00, 0.00".to_string()
        };

        let mouse_overlay = container(
            container(
                text(mouse_pos_text).size(12).color(iced::Color::WHITE),
            )
            .padding(4)
            .style(|_theme: &iced::Theme| container::Style {
                background: Some(iced::Background::Color(iced::Color::from_rgba(0.0, 0.0, 0.0, 0.5))),
                border: iced::Border { radius: 4.0.into(), ..Default::default() },
                ..Default::default()
            })
        )
        .padding(8);

        let field_stack: Element<'_, Message> = if self.replay_mode {
            let can_control_replay = !self.replay_frames.is_empty();

            let play_btn = if can_control_replay {
                button(text("Play").size(12))
                    .on_press(Message::ReplayPlay)
                    .style(button::success)
            } else {
                button(text("Play").size(12)).style(button::secondary)
            };

            let pause_btn = if can_control_replay {
                button(text("Pause").size(12))
                    .on_press(Message::ReplayPause)
                    .style(button::danger)
            } else {
                button(text("Pause").size(12)).style(button::secondary)
            };

            let status_text = if self.replay_frames.is_empty() {
                "Replay: no frames"
            } else if self.replay_is_playing {
                "Replay: playing"
            } else {
                "Replay: paused"
            };

            let frame_text = if self.replay_frames.is_empty() {
                "Frame 0/0".to_string()
            } else {
                format!("Frame {}/{}", self.replay_frame_index + 1, self.replay_frames.len())
            };

            let time_text = if self.replay_frames.is_empty() {
                "0.00s".to_string()
            } else {
                format!("{:.2}s", self.replay_frames[self.replay_frame_index].elapsed_ms as f64 / 1000.0)
            };

            let timeline: Element<'_, Message> = if can_control_replay {
                slider(
                    0..=self.replay_frames.len().saturating_sub(1) as u32,
                    self.replay_frame_index as u32,
                    Message::ReplaySeek,
                )
                .step(1u32)
                .width(Length::Fill)
                .into()
            } else {
                container(text("Load a CSV replay to enable timeline").size(11).color(iced::Color::from_rgb(0.8, 0.8, 0.8)))
                    .width(Length::Fill)
                    .into()
            };

            let replay_controls = container(
                container(column![
                    row![
                        play_btn,
                        pause_btn,
                        text(status_text).size(12).color(iced::Color::WHITE),
                        text(frame_text).size(12).color(iced::Color::from_rgb(0.7, 0.9, 1.0)),
                        text(time_text).size(12).color(iced::Color::from_rgb(0.7, 0.9, 1.0)),
                    ]
                    .spacing(8)
                    .align_y(iced::Alignment::Center),
                    timeline,
                ]
                .spacing(6))
                .padding([6, 10])
                .style(|_theme: &iced::Theme| container::Style {
                    background: Some(iced::Background::Color(iced::Color::from_rgba(0.0, 0.0, 0.0, 0.55))),
                    border: iced::Border { radius: 6.0.into(), ..Default::default() },
                    ..Default::default()
                }),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Bottom)
            .padding(8);

            iced::widget::stack![canvas, mouse_overlay, replay_controls].into()
        } else {
            iced::widget::stack![canvas, mouse_overlay].into()
        };

        let mut canvas_area = column![toolbar, field_stack];
        if self.lua_console_panel.open {
            canvas_area = canvas_area.push(self.lua_console_panel.view(Message::LuaConsoleResizeStart));
        }

        // Main content area = sidebar + canvas
        let main_row = row![
            sidebar,
            container(canvas_area)
                .width(Length::Fill)
                .height(Length::Fill),
        ];

        // Full layout
        let layout = if self.replay_mode {
            column![
                container(main_row.width(Length::Fill).height(Length::Fill))
                    .width(Length::Fill)
                    .height(Length::Fill),
            ]
        } else {
            let bottom = self.bottom_panel.view().map(Message::BottomPanel);
            column![
                container(main_row.width(Length::Fill).height(Length::Fill))
                    .width(Length::Fill)
                    .height(Length::Fill),
                bottom,
            ]
        };

        container(layout)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_panel(&self, panel: SidebarPanel) -> Element<'_, Message> {
        let content: Element<'_, Message> = match panel {
            SidebarPanel::Vision => self.vision_panel.view().map(Message::Vision),
            SidebarPanel::Radio => self.radio_panel.view().map(Message::Radio),
            SidebarPanel::Kalman => self.kalman_panel.view().map(Message::Kalman),
            SidebarPanel::Recording => self.recording_panel.view().map(Message::Recording),
            SidebarPanel::Control => self.control_panel.view().map(Message::Control),
            SidebarPanel::Charts => self.charts_panel.view().map(Message::Charts),
            SidebarPanel::LuaConsole => self.lua_console_panel.view(Message::LuaConsoleResizeStart),
        };

        container(scrollable(content))
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(4)
            .into()
    }

    fn poll_keyboard_control(&mut self) {
        if self.replay_mode || !self.control_panel.active || self.control_panel.mode != panels::control::ControlMode::Keyboard {
            self.kick_key_was_down = false;
            return;
        }

        let kick_key_down = self.key_chars.contains(&'k');
        if kick_key_down && !self.kick_key_was_down {
            let _ = self.command_tx.try_send(EngineCommand::SendKickCommand {
                id: self.control_panel.robot_id_parsed(),
                team: self.control_panel.team.to_id(),
            });
        }
        self.kick_key_was_down = kick_key_down;

        let mut vx = 0.0f64;
        let mut vy = 0.0f64;
        let mut omega = 0.0f64;

        if self.key_chars.contains(&'w') { vx += 2.0; }
        if self.key_chars.contains(&'s') { vx -= 2.0; }
        if self.key_chars.contains(&'a') { vy += 2.0; }
        if self.key_chars.contains(&'d') { vy -= 2.0; }
        if self.key_chars.contains(&'q') { omega += 4.0; }
        if self.key_chars.contains(&'e') { omega -= 4.0; }

        vx *= self.control_panel.scale_vx_parsed();
        vy *= self.control_panel.scale_vy_parsed();
        omega *= self.control_panel.scale_w_parsed();

        if vx.abs() >= 0.05 || vy.abs() >= 0.05 || omega.abs() >= 0.05 {
            let _ = self.command_tx.try_send(EngineCommand::SendRobotCommand {
                id: self.control_panel.robot_id_parsed(),
                team: self.control_panel.team.to_id(),
                vx,
                vy,
                omega,
            });
        }
    }

    fn tick_replay(&mut self) {
        self.field_data.vision_connected = true;
        self.vision_panel.connected = true;
        self.vision_panel.pps = 0;
        self.toolbar.pps = 0;
        self.field_data.lua_draw_commands.clear();

        if self.replay_frames.is_empty() {
            self.field_data.robots_blue.clear();
            self.field_data.robots_yellow.clear();
            self.field_data.ball = (0.0, 0.0);
            return;
        }

        if self.replay_is_playing {
            let now = std::time::Instant::now();
            let delta_ms = now.duration_since(self.replay_last_tick).as_millis() as u64;
            self.replay_last_tick = now;
            self.replay_elapsed_ms = self.replay_elapsed_ms.saturating_add(delta_ms);

            while self.replay_frame_index + 1 < self.replay_frames.len()
                && self.replay_frames[self.replay_frame_index + 1].elapsed_ms <= self.replay_elapsed_ms
            {
                self.replay_frame_index += 1;
            }

            if self.replay_frame_index + 1 >= self.replay_frames.len() {
                self.replay_is_playing = false;
                self.replay_elapsed_ms = self.replay_frames[self.replay_frame_index].elapsed_ms;
            }
        } else {
            self.replay_last_tick = std::time::Instant::now();
        }

        self.apply_replay_frame();
    }

    fn apply_replay_frame(&mut self) {
        if let Some(frame) = self.replay_frames.get(self.replay_frame_index) {
            self.field_data.robots_blue = frame.robots_blue.clone();
            self.field_data.robots_yellow = frame.robots_yellow.clone();
            self.field_data.ball = frame.ball;
            self.field_data.robot_trace.clear();
            self.robot_trace.clear();
        }
    }

    fn load_replay_frames(path: &str) -> Result<Vec<ReplayFrame>, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read file '{}': {e}", path))?;

        let mut by_time: BTreeMap<u64, ReplayFrameBuilder> = BTreeMap::new();

        for (line_idx, line) in content.lines().enumerate() {
            if line_idx == 0 {
                // CSV header
                continue;
            }

            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let cols: Vec<&str> = line.split(',').collect();
            if cols.len() < 11 {
                continue;
            }

            let elapsed = match cols[0].trim().parse::<f64>() {
                Ok(v) => v,
                Err(_) => continue,
            };
            let robot_id = match cols[1].trim().parse::<i32>() {
                Ok(v) => v,
                Err(_) => continue,
            };
            let team = match cols[2].trim().parse::<i32>() {
                Ok(v) => v,
                Err(_) => continue,
            };

            let vx_cmd = match cols[3].trim().parse::<f64>() {
                Ok(v) => v,
                Err(_) => continue,
            };
            let vy_cmd = match cols[4].trim().parse::<f64>() {
                Ok(v) => v,
                Err(_) => continue,
            };
            let angular_cmd = match cols[5].trim().parse::<f64>() {
                Ok(v) => v,
                Err(_) => continue,
            };
            let pos_x = match cols[6].trim().parse::<f64>() {
                Ok(v) => v,
                Err(_) => continue,
            };
            let pos_y = match cols[7].trim().parse::<f64>() {
                Ok(v) => v,
                Err(_) => continue,
            };
            let theta = match cols[8].trim().parse::<f64>() {
                Ok(v) => v,
                Err(_) => continue,
            };
            let vx_actual = match cols[9].trim().parse::<f64>() {
                Ok(v) => v,
                Err(_) => continue,
            };
            let vy_actual = match cols[10].trim().parse::<f64>() {
                Ok(v) => v,
                Err(_) => continue,
            };

            let elapsed_ms = (elapsed.max(0.0) * 1000.0).round() as u64;
            let frame = by_time.entry(elapsed_ms).or_default();

            if robot_id == -1 && team == -1 {
                frame.ball = (pos_x, pos_y);
                continue;
            }

            if robot_id < 0 {
                continue;
            }

            let robot = RobotData {
                id: robot_id as u32,
                x: pos_x,
                y: pos_y,
                theta,
                vx: vx_actual,
                vy: vy_actual,
                cmd_vx: vx_cmd,
                cmd_vy: vy_cmd,
                cmd_angular: angular_cmd,
            };

            match team {
                0 => {
                    frame.robots_blue.insert(robot.id, robot);
                }
                1 => {
                    frame.robots_yellow.insert(robot.id, robot);
                }
                _ => {}
            }
        }

        let mut frames = Vec::new();
        for (elapsed_ms, builder) in by_time {
            let mut robots_blue: Vec<RobotData> = builder.robots_blue.into_values().collect();
            let mut robots_yellow: Vec<RobotData> = builder.robots_yellow.into_values().collect();
            robots_blue.sort_by_key(|r| r.id);
            robots_yellow.sort_by_key(|r| r.id);
            frames.push(ReplayFrame {
                elapsed_ms,
                robots_blue,
                robots_yellow,
                ball: builder.ball,
            });
        }

        Ok(frames)
    }
}
