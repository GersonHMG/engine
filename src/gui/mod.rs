// gui/mod.rs — Main Iced daemon application module for the Sysmic Engine GUI

pub mod field_canvas;
pub mod sidebar;
pub mod toolbar;
pub mod bottom_panel;
pub mod panels;

use iced::widget::{column, container, row, text, scrollable};
use iced::{Element, Length, Subscription, Theme};
use iced::event::{self, Event};
use iced::keyboard;
use iced::mouse;
use iced::window;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

use crate::types::Vec2D;

use field_canvas::{FieldCanvas, FieldData, LuaDrawCommand, RobotData};
use sidebar::{Sidebar, SidebarMessage, SidebarPanel};
use toolbar::{Toolbar, ToolbarMessage};
use bottom_panel::{BottomPanel, BottomPanelMessage};
use panels::vision::{VisionPanel, VisionMessage};
use panels::radio::{RadioPanel, RadioMessage};
use panels::kalman::{KalmanPanel, KalmanMessage};
use panels::recording::{RecordingPanel, RecordingMessage, RecordingStatus};
use panels::control::{ControlPanel, ControlMessage};

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
    Point { x: f64, y: f64 },
    HighlightRobot { id: i32, team: i32 },
    Line { points: Vec<(f64, f64)> },
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
    LoadScript { path: String },
    PauseScript,
    ResumeScript,
}

// --- GUI channels ---
pub struct GuiChannels {
    pub vision_rx: mpsc::Receiver<VisionUpdate>,
    pub lua_draw_rx: mpsc::Receiver<Vec<LuaDrawCmd>>,
    pub command_tx: mpsc::Sender<EngineCommand>,
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

    // Events
    Tick,
    EventOccurred(Event),
    ScriptFileSelected(Option<String>),

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

    // Data
    field_data: FieldData,
    robot_trace: Vec<(f64, f64)>,
    last_vision_time: std::time::Instant,
    last_pps_time: std::time::Instant,

    // Channels (wrapped in Arc<Mutex> for Iced)
    vision_rx: Arc<Mutex<mpsc::Receiver<VisionUpdate>>>,
    lua_draw_rx: Arc<Mutex<mpsc::Receiver<Vec<LuaDrawCmd>>>>,
    command_tx: mpsc::Sender<EngineCommand>,

    // Keyboard state for manual control
    key_chars: std::collections::HashSet<char>,

    // Mouse state
    last_cursor_position: Option<iced::Point>,

    // Window management
    main_window_id: window::Id,
    panel_windows: HashMap<SidebarPanel, window::Id>,
    window_to_panel: HashMap<window::Id, SidebarPanel>,
}

impl EngineApp {
    /// Boot function for iced::daemon — opens the main window and returns initial state + tasks
    pub fn boot(channels: GuiChannels) -> (Self, iced::Task<Message>) {
        let (main_id, open_task) = window::open(window::Settings {
            size: iced::Size::new(900.0, 800.0),
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
            field_data: FieldData::default(),
            robot_trace: Vec::new(),
            last_vision_time: std::time::Instant::now(),
            last_pps_time: std::time::Instant::now(),
            vision_rx: Arc::new(Mutex::new(channels.vision_rx)),
            lua_draw_rx: Arc::new(Mutex::new(channels.lua_draw_rx)),
            command_tx: channels.command_tx,
            key_chars: std::collections::HashSet::new(),
            last_cursor_position: None,
            main_window_id: main_id,
            panel_windows: HashMap::new(),
            window_to_panel: HashMap::new(),
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
            return iced::Task::none();
        }

        let (id, task) = window::open(window::Settings {
            size: iced::Size::new(300.0, 420.0),
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
        match message {
            // --- Sidebar ---
            Message::Sidebar(SidebarMessage::TogglePanel(panel)) => {
                if self.panel_windows.contains_key(&panel) {
                    return self.close_panel_window(panel);
                } else {
                    return self.open_panel_window(panel);
                }
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
                    self.toolbar.script_status = toolbar::ScriptStatus::Paused;
                } else {
                    let _ = self.command_tx.try_send(EngineCommand::ResumeScript);
                    self.toolbar.script_status = toolbar::ScriptStatus::Running;
                }
            }
            Message::Toolbar(ToolbarMessage::ReloadScript) => {
                if !self.toolbar.script_path.is_empty() {
                    let path = self.toolbar.script_path.clone();
                    let _ = self.command_tx.try_send(EngineCommand::LoadScript { path });
                    self.toolbar.script_status = toolbar::ScriptStatus::Loaded;
                }
            }

            Message::ScriptFileSelected(Some(path)) => {
                let _ = self.command_tx.try_send(EngineCommand::LoadScript { path: path.clone() });
                self.toolbar.script_path = path;
                self.toolbar.script_status = toolbar::ScriptStatus::Loaded;
            }
            Message::ScriptFileSelected(None) => {}

            // --- Bottom Panel ---
            Message::BottomPanel(BottomPanelMessage::ToggleCapture) => {
                self.bottom_panel.capturing = !self.bottom_panel.capturing;
            }
            Message::BottomPanel(BottomPanelMessage::ToggleTrace) => {
                self.bottom_panel.trace_on = !self.bottom_panel.trace_on;
                self.field_data.robot_trace.clear();
                self.robot_trace.clear();
            }
            Message::BottomPanel(BottomPanelMessage::ToggleVectors) => {
                self.bottom_panel.vectors_on = !self.bottom_panel.vectors_on;
                self.field_data.vis_velocities = self.bottom_panel.vectors_on;
            }
            Message::BottomPanel(BottomPanelMessage::TeamSelected(team)) => {
                self.bottom_panel.control_team = team;
                self.control_panel.team = team;
            }
            Message::BottomPanel(BottomPanelMessage::RobotIdChanged(id)) => {
                self.bottom_panel.control_robot_id = id.clone();
                self.control_panel.robot_id = id;
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

            // --- Control Panel ---
            Message::Control(ControlMessage::ModeSelected(mode)) => {
                self.control_panel.mode = mode;
            }
            Message::Control(ControlMessage::ActiveToggled(val)) => {
                self.control_panel.active = val;
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

                        // Charts
                        let ctrl_id = self.control_panel.robot_id_parsed();
                        let ctrl_team = self.control_panel.team.to_id();
                        let robots = if ctrl_team == 0 { &update.robots_blue } else { &update.robots_yellow };
                        if let Some(target) = robots.iter().find(|r| r.id == ctrl_id as u32) {
                            if self.bottom_panel.capturing {
                                self.bottom_panel.chart_data.push_vel(target.cmd_vx, target.cmd_vy, target.cmd_angular);
                                self.bottom_panel.chart_data.push_pos(target.x, target.y, target.theta);
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
                                LuaDrawCmd::Point { x, y } => LuaDrawCommand::Point { x: *x, y: *y },
                                LuaDrawCmd::HighlightRobot { id, team } => LuaDrawCommand::HighlightRobot { id: *id, team: *team },
                                LuaDrawCmd::Line { points } => LuaDrawCommand::Line { points: points.clone() },
                            })
                            .collect();
                    }
                }

                self.field_canvas.request_redraw();
            }

            // --- Global events (keyboard + mouse) ---
            Message::EventOccurred(event) => {
                match event {
                    Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) => {
                        if let keyboard::Key::Character(c) = &key {
                            let ch = c.chars().next().unwrap_or(' ');
                            self.key_chars.insert(ch);
                        }
                    }
                    Event::Keyboard(keyboard::Event::KeyReleased { key, .. }) => {
                        if let keyboard::Key::Character(c) = &key {
                            let ch = c.chars().next().unwrap_or(' ');
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
                    Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                        if let Some(pos) = self.last_cursor_position {
                            self.field_canvas.handle_drag_start(pos);
                        }
                    }
                    Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                        self.field_canvas.handle_drag_end();
                    }
                    Event::Mouse(mouse::Event::CursorMoved { position }) => {
                        self.last_cursor_position = Some(iced::Point::new(position.x, position.y));
                        self.field_canvas.handle_drag_move(iced::Point::new(position.x, position.y));
                        self.field_canvas.update_mouse_pos(iced::Point::new(position.x, position.y));
                    }
                    _ => {}
                }
            }


            // --- Window events ---
            Message::WindowOpened(_id) => {}
            Message::WindowClosed(id) => {
                if id == self.main_window_id {
                    let panel_ids: Vec<window::Id> = self.panel_windows.values().cloned().collect();
                    let mut tasks: Vec<iced::Task<Message>> = panel_ids.iter().map(|wid| window::close(*wid)).collect();
                    tasks.push(iced::exit());
                    return iced::Task::batch(tasks);
                } else {
                    if let Some(panel) = self.window_to_panel.remove(&id) {
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
        let sidebar = self.sidebar.view().map(Message::Sidebar);

        // Toolbar
        let toolbar = self.toolbar.view().map(Message::Toolbar);

        // Field canvas
        let canvas: Element<'_, Message> = self.field_canvas.view(&self.field_data);

        // Mouse coords overlay
        let mouse_pos_text = if let Some((x, y)) = self.field_canvas.mouse_field_pos() {
            format!("{:.2}, {:.2}", x, y)
        } else {
            "0.00, 0.00".to_string()
        };

        let canvas_area = column![
            toolbar,
            iced::widget::stack![
                canvas,
                container(
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
                .padding(8),
            ],
        ];

        // Main content area = sidebar + canvas
        let main_row = row![
            sidebar,
            container(canvas_area)
                .width(Length::Fill)
                .height(Length::Fill),
        ];

        // Bottom panel
        let bottom = self.bottom_panel.view().map(Message::BottomPanel);

        // Full layout
        let layout = column![
            container(main_row.width(Length::Fill).height(Length::Fill))
                .width(Length::Fill)
                .height(Length::Fill),
            bottom,
        ];

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
        };

        container(scrollable(content))
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(4)
            .into()
    }

    fn poll_keyboard_control(&mut self) {
        if !self.control_panel.active || self.control_panel.mode != panels::control::ControlMode::Keyboard {
            return;
        }

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

        if vx.abs() < 0.05 && vy.abs() < 0.05 && omega.abs() < 0.05 {
            return;
        }

        let _ = self.command_tx.try_send(EngineCommand::SendRobotCommand {
            id: self.control_panel.robot_id_parsed(),
            team: self.control_panel.team.to_id(),
            vx,
            vy,
            omega,
        });
    }
}
