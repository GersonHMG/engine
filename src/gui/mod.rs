// gui/mod.rs — Main Iced daemon application module for the Sysmic Engine GUI

pub mod field_canvas;
pub mod sidebar;
pub mod toolbar;
pub mod bottom_panel;
pub mod panels;
pub mod lua_console;
pub mod recorder;
pub mod replay;
pub mod robot;
pub mod teleport_menu;
pub mod theme;
pub mod types;

pub use types::{EngineCommand, GuiChannels, LuaDrawCmd, LuaScriptStatusUpdate, Message, RobotUpdateData, SimulatorSettings, VisionUpdate};

use iced::widget::{column, container, pick_list, row, scrollable, slider, text, text_input};
use iced::widget::operation::snap_to_end;
use iced::{Element, Length, Subscription, Theme};
use iced::event::{self, Event};
use iced::keyboard;
use iced::mouse;
use iced::window;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tracing::warn;

use crate::config::FieldConfig;
use crate::sender::simulator::scenario;

use field_canvas::{FieldCanvas, FieldData, FieldMessage, LuaDrawCommand, RobotData};
use recorder::Recorder;
use replay::ReplayState;
use sidebar::{Sidebar, SidebarMessage, SidebarPanel};
use teleport_menu::{TeleportMenu, TeleportMenuMessage};
use theme::Btn;
use toolbar::{Toolbar, ToolbarMessage};
use bottom_panel::{BottomPanel, BottomPanelMessage};
use lua_console::LuaConsolePanel;
use panels::vision::{VisionPanel, VisionMessage};
use panels::radio::{RadioPanel, RadioMessage};
use panels::kalman::{KalmanPanel, KalmanMessage};
use panels::recording::{RecordingPanel, RecordingMessage, RecordingStatus};
use panels::control::{ControlPanel, ControlMessage};
use panels::charts::{export_csv_async, ChartsMessage, ChartsPanel, DataSample};

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
    /// Simulated time of the last field update, s, while the simulator runs.
    sim_time: Option<f64>,
    last_pps_time: std::time::Instant,

    // Channels (wrapped in Arc<Mutex> for Iced)
    vision_rx: Arc<Mutex<mpsc::Receiver<VisionUpdate>>>,
    lua_draw_rx: Arc<Mutex<mpsc::Receiver<Vec<LuaDrawCmd>>>>,
    lua_status_rx: Arc<Mutex<mpsc::Receiver<LuaScriptStatusUpdate>>>,
    lua_log_rx: Arc<Mutex<mpsc::Receiver<String>>>,
    simulator_rx: Arc<Mutex<mpsc::Receiver<SimulatorSettings>>>,
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
    replay: ReplayState,
    // Name typed for the scenario saved from the replay (empty = a name from the frame)
    replay_scenario_name: String,
    // Outcome of the last scenario save, shown under the replay controls
    replay_scenario_status: Option<Result<String, String>>,
    // Toolbar recorder (REC / REPLAY)
    recorder: Recorder,
    // Live Lua drawings, kept up to date while replay mode shows recorded ones
    live_draws: Vec<LuaDrawCommand>,
    // Right-click menu on the field, while open
    teleport_menu: Option<TeleportMenu>,
}

fn app_icon() -> Option<window::Icon> {
    // Best-effort icon load; missing/invalid icon falls back to default.
    window::icon::from_file("icons/sysmic.ico").ok()
}

impl EngineApp {
    /// Boot function for iced::daemon — opens the main window and returns initial state + tasks
    pub fn boot(
        channels: GuiChannels,
        field_config: FieldConfig,
        simulator: SimulatorSettings,
    ) -> (Self, iced::Task<Message>) {
        let (main_id, open_task) = window::open(window::Settings {
            size: iced::Size::new(900.0, 600.0),
            icon: app_icon(),
            ..Default::default()
        });

        let app = Self {
            field_canvas: FieldCanvas::new(),
            sidebar: Sidebar::new(),
            toolbar: Toolbar::new(simulator),
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
            sim_time: None,
            last_pps_time: std::time::Instant::now(),
            vision_rx: Arc::new(Mutex::new(channels.vision_rx)),
            lua_draw_rx: Arc::new(Mutex::new(channels.lua_draw_rx)),
            lua_status_rx: Arc::new(Mutex::new(channels.lua_status_rx)),
            lua_log_rx: Arc::new(Mutex::new(channels.lua_log_rx)),
            simulator_rx: Arc::new(Mutex::new(channels.simulator_rx)),
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
            replay: ReplayState::default(),
            replay_scenario_name: String::new(),
            replay_scenario_status: None,
            recorder: Recorder::default(),
            live_draws: Vec::new(),
            teleport_menu: None,
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
        theme::app_theme()
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
            icon: app_icon(),
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

    /// Switches to replay mode: closes the panel windows, stops recording and
    /// keeps the live drawings aside until replay mode ends.
    fn enter_replay_mode(&mut self) -> iced::Task<Message> {
        self.recorder.stop();
        self.replay.set_enabled(true);
        self.live_draws = std::mem::take(&mut self.field_data.lua_draw_commands);

        let panel_ids: Vec<window::Id> = self.panel_windows.values().cloned().collect();
        self.panel_windows.clear();
        self.window_to_panel.clear();
        self.sidebar.active_panel = None;
        self.lua_console_panel.open = false;
        self.key_chars.clear();
        self.kick_key_was_down = false;

        let tasks: Vec<iced::Task<Message>> = panel_ids.into_iter().map(window::close).collect();
        iced::Task::batch(tasks)
    }

    /// Back to the live view, with the live drawings.
    fn exit_replay_mode(&mut self) {
        self.replay.set_enabled(false);
        self.field_data.lua_draw_commands = std::mem::take(&mut self.live_draws);
    }

    /// Plays the last recording from its start, in replay mode.
    fn play_last_recording(&mut self) -> iced::Task<Message> {
        if self.recorder.is_recording() || !self.recorder.has_recording() {
            return iced::Task::none();
        }
        let task = if self.replay.enabled { iced::Task::none() } else { self.enter_replay_mode() };
        let label = format!("Last recording ({:.1} s)", self.recorder.last_duration_s());
        let frames = self.recorder.last_recording().to_vec();
        self.replay.load_recording(frames, label, &mut self.field_data, &mut self.robot_trace);
        self.replay.start_playback(&mut self.field_data, &mut self.robot_trace);
        task
    }

    /// Takes the newest Lua drawings sent by the engine, if any arrived.
    fn drain_lua_draws(&mut self) -> Option<Vec<LuaDrawCommand>> {
        let mut latest = None;
        if let Ok(mut rx) = self.lua_draw_rx.try_lock() {
            while let Ok(cmds) = rx.try_recv() {
                latest = Some(cmds);
            }
        }
        latest.map(|cmds| cmds.iter().map(to_field_draw).collect())
    }

    pub fn update(&mut self, message: Message) -> iced::Task<Message> {
        if self.replay.enabled {
            match &message {
                Message::Sidebar(SidebarMessage::ToggleReplayMode)
                | Message::Toolbar(ToolbarMessage::PlayRecording)
                | Message::Toolbar(ToolbarMessage::ToggleDraws)
                | Message::ReplayFilePick
                | Message::ReplayFileSelected(_)
                | Message::ReplayTogglePlay
                | Message::ReplayScenarioNameChanged(_)
                | Message::ReplaySaveScenario
                | Message::ReplaySeek(_)
                | Message::ReplaySpeedSelected(_)
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
                if self.replay.enabled {
                    self.exit_replay_mode();
                } else {
                    return self.enter_replay_mode();
                }
            }
            Message::Sidebar(SidebarMessage::TogglePanel(panel)) => {
                if self.replay.enabled {
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
                if let Err(e) = self.replay.load_and_apply(path, &mut self.field_data, &mut self.robot_trace) {
                    warn!("Replay: failed to load CSV: {e}");
                }
            }
            Message::ReplayFileSelected(None) => {}
            Message::ReplayTogglePlay => {
                self.replay.toggle_playback(&mut self.field_data, &mut self.robot_trace);
            }
            Message::ReplayScenarioNameChanged(name) => {
                self.replay_scenario_name = name;
            }
            Message::ReplaySaveScenario => {
                self.replay.pause();
                let name = match self.replay_scenario_name.trim() {
                    "" => self.replay.default_scenario_name(),
                    name => name.to_string(),
                };
                self.replay_scenario_status = Some(match self.replay.current_scenario() {
                    Some(setup) => match scenario::save(&name, &setup) {
                        Ok(path) => Ok(format!("Saved {} — load it with `sim scenario load {name}`", path.display())),
                        Err(e) => Err(e),
                    },
                    None => Err("no frame to save".to_string()),
                });
            }
            Message::ReplaySeek(index) => {
                self.replay.seek(index, &mut self.field_data, &mut self.robot_trace);
            }
            Message::ReplaySpeedSelected(speed) => {
                self.replay.speed = speed.0;
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
            Message::Toolbar(ToolbarMessage::ToggleRecording) => {
                if self.recorder.is_recording() {
                    self.recorder.stop();
                } else {
                    self.recorder.start();
                }
            }
            Message::Toolbar(ToolbarMessage::PlayRecording) => {
                return self.play_last_recording();
            }
            // --- Console command line ---
            Message::ConsoleInputChanged(input) => {
                self.lua_console_panel.set_input(input);
            }
            Message::ConsoleClear => {
                self.lua_console_panel.clear();
            }
            Message::ConsoleSubmit => {
                if let Some(line) = self.lua_console_panel.take_input() {
                    let _ = self.command_tx.try_send(EngineCommand::ConsoleCommand(line));
                }
            }

            // --- Right-click teleport menu ---
            Message::Field(FieldMessage::RightClicked { screen, field }) => {
                if !self.replay.enabled {
                    self.teleport_menu = Some(TeleportMenu { screen, field });
                }
            }
            Message::Field(FieldMessage::RobotClicked { id, team }) => {
                self.field_data.selected_robot = Some((id, team));
            }
            Message::TeleportMenu(choice) => {
                let Some(menu) = self.teleport_menu.take() else {
                    return iced::Task::none();
                };
                let (x, y) = menu.field;
                let command = match choice {
                    TeleportMenuMessage::Ball => EngineCommand::TeleportBall { x, y },
                    TeleportMenuMessage::SelectedRobot => {
                        let Some((id, team)) = self.field_data.selected_robot else {
                            return iced::Task::none();
                        };
                        // Keep the robot's current heading.
                        let orientation = self.team_robots(team).iter().find(|r| r.id == id).map_or(0.0, |r| r.theta);
                        EngineCommand::TeleportRobot { id: id as i32, team, x, y, orientation }
                    }
                    TeleportMenuMessage::AddRobot { team } => {
                        let Some(id) = self.free_robot_id(team) else {
                            return iced::Task::none();
                        };
                        // Teleporting a robot that is not on the field adds it
                        // (simulator) or turns it on (grSim). It faces the
                        // opponent's goal.
                        let orientation = if team == 0 { 0.0 } else { std::f64::consts::PI };
                        EngineCommand::TeleportRobot { id: id as i32, team, x, y, orientation }
                    }
                };
                let _ = self.command_tx.try_send(command);
            }
            Message::Toolbar(ToolbarMessage::ToggleDraws) => {
                self.toolbar.show_draws = !self.toolbar.show_draws;
                self.field_data.show_lua_draws = self.toolbar.show_draws;
                self.field_canvas.request_redraw();
            }
            Message::Toolbar(ToolbarMessage::ToggleSimulator) => {
                self.toolbar.simulator.enabled = !self.toolbar.simulator.enabled;
                let _ = self.command_tx.try_send(EngineCommand::SetSimulator(self.toolbar.simulator));
            }
            Message::Toolbar(ToolbarMessage::SimSpeedSelected(speed)) => {
                self.toolbar.simulator.speed = speed.0;
                let _ = self.command_tx.try_send(EngineCommand::SetSimulator(self.toolbar.simulator));
            }
            Message::Toolbar(ToolbarMessage::ScenarioMenuOpened) => {
                self.toolbar.scenarios = scenario::list();
            }
            Message::Toolbar(ToolbarMessage::ScenarioSelected(name)) => {
                let _ = self
                    .command_tx
                    .try_send(EngineCommand::ConsoleCommand(format!("sim scenario load {name}")));
            }
            Message::Toolbar(ToolbarMessage::AddRobot(team)) => {
                if let Some(id) = self.free_robot_id(team) {
                    // Without a position, the robot goes to a free spot on its half.
                    let command = format!("sim add {} {id}", team_word(team));
                    let _ = self.command_tx.try_send(EngineCommand::ConsoleCommand(command));
                }
            }
            Message::Toolbar(ToolbarMessage::RemoveRobot(team)) => {
                if let Some(id) = self.team_robots(team).iter().map(|r| r.id).max() {
                    let command = format!("sim remove {} {id}", team_word(team));
                    let _ = self.command_tx.try_send(EngineCommand::ConsoleCommand(command));
                }
            }
            Message::ScriptFileSelected(Some(path)) => {
                let _ = self.command_tx.try_send(EngineCommand::LoadScript { path: path.clone() });
                self.toolbar.script_path = path;
            }
            Message::ScriptFileSelected(None) => {}

            // --- Bottom Panel ---
            Message::BottomPanel(BottomPanelMessage::SetManualControl(val)) => {
                self.bottom_panel.manual_control_on = val;
                self.control_panel.active = val;
                if val {
                    let id = self.bottom_panel.control_robot_id.parse::<u32>().unwrap_or(0);
                    let team = self.bottom_panel.control_team.to_id();
                    self.field_data.highlight_robot = Some((id, team));
                } else {
                    self.field_data.highlight_robot = None;
                }
            }
            Message::BottomPanel(BottomPanelMessage::SetTrace(val)) => {
                self.bottom_panel.trace_on = val;
                self.field_data.robot_trace.clear();
                self.robot_trace.clear();
            }
            Message::BottomPanel(BottomPanelMessage::SetVectors(val)) => {
                self.bottom_panel.vectors_on = val;
                self.field_data.vis_velocities = val;
            }
            
            Message::BottomPanel(BottomPanelMessage::IncrementRobotId) => {
                let new_id = (self.bottom_panel.control_robot_id.parse::<i32>().unwrap_or(0) + 1).min(12);
                let s = new_id.to_string();
                self.bottom_panel.control_robot_id = s.clone();
                self.control_panel.robot_id = s.clone();
                if self.bottom_panel.manual_control_on {
                    self.field_data.highlight_robot = Some((new_id as u32, self.bottom_panel.control_team.to_id()));
                }
            }
            Message::BottomPanel(BottomPanelMessage::DecrementRobotId) => {
                let new_id = (self.bottom_panel.control_robot_id.parse::<i32>().unwrap_or(0) - 1).max(0);
                let s = new_id.to_string();
                self.bottom_panel.control_robot_id = s.clone();
                self.control_panel.robot_id = s.clone();
                if self.bottom_panel.manual_control_on {
                    self.field_data.highlight_robot = Some((new_id as u32, self.bottom_panel.control_team.to_id()));
                }
            }
            Message::BottomPanel(BottomPanelMessage::TeamSelected(team)) => {
                self.bottom_panel.control_team = team;
                self.control_panel.team = team;
                if self.bottom_panel.manual_control_on {
                    let id = self.bottom_panel.control_robot_id.parse::<u32>().unwrap_or(0);
                    self.field_data.highlight_robot = Some((id, team.to_id()));
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
                if self.replay.enabled {
                    self.vision_panel.connected = true;
                    self.vision_panel.pps = 0;
                    self.bottom_panel.pps = 0;
                    if let Some(draws) = self.drain_lua_draws() {
                        self.live_draws = draws;
                    }
                    self.replay.tick(&mut self.field_data, &mut self.robot_trace);
                    self.field_canvas.request_redraw();
                    return iced::Task::none();
                }

                let elapsed = self.last_vision_time.elapsed();
                self.field_data.vision_connected = elapsed.as_millis() < 1000;
                if !self.field_data.vision_connected {
                    self.vision_panel.connected = false;
                    self.vision_panel.pps = 0;
                    self.bottom_panel.pps = 0;
                }

                self.poll_keyboard_control();

                // Throttle PPS sparkline pushes to every 250ms (~4Hz instead of 60Hz)
                let mut spark_push = false;
                if self.last_pps_time.elapsed().as_millis() > 250 {
                    self.last_pps_time = std::time::Instant::now();
                    spark_push = true;
                }
                
                if spark_push && !self.field_data.vision_connected {
                    self.bottom_panel.push_pps(0);
                }

                // Drain vision channel
                if let Ok(mut rx) = self.vision_rx.try_lock() {
                    while let Ok(update) = rx.try_recv() {
                        self.last_vision_time = std::time::Instant::now();
                        self.vision_panel.connected = true;
                        self.vision_panel.pps = update.pps;
                        self.bottom_panel.pps = update.pps;
                        self.sim_time = update.sim_time;
                        if spark_push {
                            self.bottom_panel.push_pps(update.pps);
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

                // Drain lua draw channel (the engine sends drawings on each tick that
                // ran process() and on script loads, so they stay while paused)
                if let Some(draws) = self.drain_lua_draws() {
                    self.field_data.lua_draw_commands = draws;
                }

                // Keep the SIM button and speed in step with the engine
                if let Ok(mut rx) = self.simulator_rx.try_lock() {
                    while let Ok(settings) = rx.try_recv() {
                        self.toolbar.simulator = settings;
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

                // Record what the field shows now (robots, ball, drawings)
                if !self.toolbar.simulator.enabled {
                    self.sim_time = None;
                }
                self.recorder.capture(&self.field_data, self.sim_time);
                self.toolbar.recording_s = self.recorder.elapsed_s();
                self.toolbar.has_recording = self.recorder.has_recording();

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
                        // A click outside the teleport menu closes it.
                        self.teleport_menu = None;
                        if let Some(pos) = self.last_cursor_position {
                            if self.lua_console_panel.open && !self.replay.enabled {
                                let bottom_height = if self.replay.enabled { 0.0 } else { 52.0 };
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
                        if key == keyboard::Key::Named(keyboard::key::Named::Escape) {
                            self.teleport_menu = None;
                            self.field_data.selected_robot = None;
                        }
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
        let robot_counts = [self.field_data.robots_blue.len(), self.field_data.robots_yellow.len()];

        // Sidebar
        let open_panels: Vec<SidebarPanel> = self.panel_windows.keys().copied().collect();
        let sidebar = self
            .sidebar
            .view(&open_panels, self.replay.enabled, self.lua_console_panel.open)
            .map(Message::Sidebar);

        // Toolbar
        let toolbar: Element<'_, Message> = if self.replay.enabled {
            let replay_file_name = if let Some(label) = &self.replay.recording_label {
                label.clone()
            } else {
                self.replay
                    .file_path
                    .as_ref()
                    .map(|p| p.replace('\\', "/").rsplit('/').next().unwrap_or("No file").to_string())
                    .unwrap_or_else(|| "No CSV selected".to_string())
            };

            let replay_pill = container(text("REPLAY").size(11).font(theme::MONO_BOLD).color(theme::ON_RED))
                .padding([2, 8])
                .style(theme::pill(theme::RED));

            let open_btn = toolbar::label_button("Open CSV".to_string(), Btn::Neutral, Some(Message::ReplayFilePick));
            let last_recording_btn = toolbar::label_button(
                "Last recording".to_string(),
                Btn::Neutral,
                self.recorder.has_recording().then_some(Message::Toolbar(ToolbarMessage::PlayRecording)),
            );
            let draws_btn = toolbar::draws_button(self.toolbar.show_draws, Message::Toolbar(ToolbarMessage::ToggleDraws));

            let speed_list = pick_list(
                replay::REPLAY_SPEEDS,
                Some(replay::ReplaySpeed(self.replay.speed)),
                Message::ReplaySpeedSelected,
            )
            .width(64)
            .font(theme::MONO)
            .text_size(12);

            let exit_btn = toolbar::label_button(
                "Exit replay".to_string(),
                Btn::Neutral,
                Some(Message::Sidebar(SidebarMessage::ToggleReplayMode)),
            );

            container(
                row![
                    replay_pill,
                    text(replay_file_name).size(12).font(theme::MONO).color(theme::TEXT_2),
                    theme::divider(),
                    open_btn,
                    last_recording_btn,
                    draws_btn,
                    text("Speed").size(11).color(theme::MUTED),
                    speed_list,
                    iced::widget::Space::new().width(Length::Fill),
                    exit_btn,
                ]
                .spacing(6)
                .align_y(iced::Alignment::Center),
            )
            .width(Length::Fill)
            .height(36)
            .padding([0, 8])
            .center_y(36)
            .style(theme::bar(theme::REPLAY_BAR))
            .into()
        } else {
            self.toolbar.view(robot_counts).map(Message::Toolbar)
        };

        // Field canvas
        let canvas: Element<'_, Message> = self
            .field_canvas
            .view(&self.field_data, Message::Field);

        // Mouse coords overlay, while the cursor is over the field
        let mouse_overlay: Element<'_, Message> = match self.field_canvas.mouse_field_pos() {
            Some((x, y)) => container(
                container(text(format!("{x:.2}, {y:.2} m")).size(11).font(theme::MONO))
                    .padding([3, 7])
                    .style(theme::overlay_box),
            )
            .padding(8)
            .into(),
            None => iced::widget::Space::new().into(),
        };

        let field_stack: Element<'_, Message> = if self.replay.enabled {
            let can_control_replay = self.replay.has_frames();

            let play_pause_btn = if self.replay.is_playing {
                toolbar::label_button("❚❚ Pause".to_string(), Btn::Neutral, Some(Message::ReplayTogglePlay))
            } else {
                toolbar::label_button(
                    "▶ Play".to_string(),
                    Btn::Play,
                    can_control_replay.then_some(Message::ReplayTogglePlay),
                )
            };

            let scenario_input = text_input(&self.replay.default_scenario_name(), &self.replay_scenario_name)
                .on_input(Message::ReplayScenarioNameChanged)
                .on_submit(Message::ReplaySaveScenario)
                .size(12)
                .padding([4, 8])
                .style(theme::input)
                .width(Length::Fixed(180.0));

            let save_btn = toolbar::label_button(
                "Save state".to_string(),
                Btn::Neutral,
                can_control_replay.then_some(Message::ReplaySaveScenario),
            );

            let frame_text = if self.replay.frames.is_empty() {
                "frame 0/0".to_string()
            } else {
                format!("frame {}/{}", self.replay.frame_index + 1, self.replay.frames.len())
            };

            let time_text = if self.replay.frames.is_empty() {
                "0.00 s".to_string()
            } else {
                format!("{:.2} s", self.replay.current_frame_elapsed_ms() as f64 / 1000.0)
            };

            let timeline: Element<'_, Message> = if can_control_replay {
                slider(
                    0..=self.replay.frames.len().saturating_sub(1) as u32,
                    self.replay.frame_index as u32,
                    Message::ReplaySeek,
                )
                .step(1u32)
                .width(Length::Fill)
                .into()
            } else {
                container(text("Load a CSV replay to enable the timeline").size(11).color(theme::MUTED))
                    .width(Length::Fill)
                    .into()
            };

            let replay_controls = container(
                container(column![
                    row![
                        play_pause_btn,
                        text(time_text).size(12).font(theme::MONO_BOLD),
                        text(frame_text).size(11).font(theme::MONO).color(theme::MUTED),
                        iced::widget::Space::new().width(Length::Fill),
                        scenario_input,
                        save_btn,
                    ]
                    .spacing(8)
                    .align_y(iced::Alignment::Center),
                    timeline,
                ]
                .extend(self.replay_scenario_status.as_ref().map(|status| match status {
                    Ok(msg) => text(msg.as_str()).size(11).color(theme::GREEN).into(),
                    Err(msg) => text(msg.as_str()).size(11).color(theme::RED_FG).into(),
                }))
                .spacing(6))
                .padding([8, 10])
                .style(theme::overlay_box),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Bottom)
            .padding(10);

            let frame = container(iced::widget::Space::new().width(Length::Fill).height(Length::Fill))
                .style(theme::replay_frame);

            iced::widget::stack![canvas, frame, mouse_overlay, replay_controls].into()
        } else if let Some(menu) = &self.teleport_menu {
            let menu = menu
                .view(
                    self.field_data.selected_robot,
                    [self.free_robot_id(0).is_some(), self.free_robot_id(1).is_some()],
                )
                .map(Message::TeleportMenu);
            iced::widget::stack![canvas, mouse_overlay, menu].into()
        } else {
            iced::widget::stack![canvas, mouse_overlay].into()
        };

        let mut canvas_area = column![toolbar, field_stack];
        if self.lua_console_panel.open {
            canvas_area = canvas_area.push(self.lua_console_panel.view(Message::LuaConsoleResizeStart, Message::ConsoleInputChanged, Message::ConsoleSubmit, Message::ConsoleClear));
        }

        // Main content area = sidebar + canvas
        let main_row = row![
            sidebar,
            container(canvas_area)
                .width(Length::Fill)
                .height(Length::Fill),
        ];

        // Full layout
        let layout = if self.replay.enabled {
            column![
                container(main_row.width(Length::Fill).height(Length::Fill))
                    .width(Length::Fill)
                    .height(Length::Fill),
            ]
        } else {
            let sim = self.toolbar.simulator.enabled.then(|| bottom_panel::SimSummary {
                speed: toolbar::SimSpeed(self.toolbar.simulator.speed).to_string(),
                robots: robot_counts,
            });
            let bottom = self.bottom_panel.view(sim).map(Message::BottomPanel);
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
            SidebarPanel::LuaConsole => self.lua_console_panel.view(Message::LuaConsoleResizeStart, Message::ConsoleInputChanged, Message::ConsoleSubmit, Message::ConsoleClear),
        };

        container(scrollable(content))
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(4)
            .into()
    }

    /// Robots on the field of one team (0 = blue, 1 = yellow).
    fn team_robots(&self, team: i32) -> &[RobotData] {
        if team == 0 { &self.field_data.robots_blue } else { &self.field_data.robots_yellow }
    }

    /// Lowest robot id of a team that is not on the field, if any is left.
    fn free_robot_id(&self, team: i32) -> Option<u32> {
        const MAX_ROBOTS_PER_TEAM: u32 = 16;
        let robots = self.team_robots(team);
        (0..MAX_ROBOTS_PER_TEAM).find(|id| robots.iter().all(|r| r.id != *id))
    }

    fn poll_keyboard_control(&mut self) {
        if self.replay.enabled || !self.control_panel.active || self.control_panel.mode != panels::control::ControlMode::Keyboard {
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

}

/// Team name as the `sim` console commands take it (0 = blue, 1 = yellow).
fn team_word(team: i32) -> &'static str {
    if team == 0 { "blue" } else { "yellow" }
}

/// A Lua drawing from the engine, in the field canvas's format.
fn to_field_draw(cmd: &LuaDrawCmd) -> LuaDrawCommand {
    match cmd {
        LuaDrawCmd::Point { x, y, draw_x, color } => LuaDrawCommand::Point { x: *x, y: *y, draw_x: *draw_x, color: *color },
        LuaDrawCmd::HighlightRobot { id, team } => LuaDrawCommand::HighlightRobot { id: *id, team: *team },
        LuaDrawCmd::Line { points, draw_points_between, color } => LuaDrawCommand::Line {
            points: points.clone(),
            draw_points_between: *draw_points_between,
            color: *color,
        },
        LuaDrawCmd::Text { x, y, text, color } => LuaDrawCommand::Text { x: *x, y: *y, text: text.clone(), color: *color },
    }
}
