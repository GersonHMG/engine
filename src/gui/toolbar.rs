// gui/toolbar.rs — Script toolbar (load/play/reload + status, record/replay/draws,
// simulator switch with team steppers, scenario menu and speed)

use super::theme::{self, bar, btn, btn_fg, divider, dot, group, icon, pill, Btn, Icon};
use super::types::SimulatorSettings;
use iced::widget::{button, container, pick_list, row, text, Space};
use iced::{Alignment, Element, Length};
use std::fmt;

/// Speeds offered for the simulator; 0 = as fast as possible.
const SIM_SPEEDS: [SimSpeed; 5] = [SimSpeed(1.0), SimSpeed(2.0), SimSpeed(5.0), SimSpeed(10.0), SimSpeed(0.0)];

#[derive(Debug, Clone)]
pub enum ToolbarMessage {
    LoadScript,
    ToggleScript,
    ReloadScript,
    /// Starts or stops recording the field (robots, ball, Lua drawings).
    ToggleRecording,
    /// Plays the last recording in replay mode.
    PlayRecording,
    /// Shows or hides the Lua drawings on the field.
    ToggleDraws,
    ToggleSimulator,
    SimSpeedSelected(SimSpeed),
    /// The scenario menu was opened; its list of saved scenarios is refreshed.
    ScenarioMenuOpened,
    /// Loads a saved scenario into the simulator.
    ScenarioSelected(String),
    /// Adds a robot of this team (0 = blue, 1 = yellow) to the simulator.
    AddRobot(i32),
    /// Removes the highest-id robot of this team from the simulator.
    RemoveRobot(i32),
}

/// Simulator speed as shown in the speed menu.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SimSpeed(pub f64);

impl fmt::Display for SimSpeed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0 == 0.0 {
            write!(f, "Max")
        } else {
            write!(f, "{}x", self.0)
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScriptStatus {
    NoScript,
    Running,
    Paused,
    Failed,
}

pub struct Toolbar {
    pub script_path: String,
    pub script_status: ScriptStatus,
    pub simulator: SimulatorSettings,
    /// Seconds recorded so far, while recording.
    pub recording_s: Option<f64>,
    /// Whether there is a finished recording to replay.
    pub has_recording: bool,
    /// Whether the Lua drawings are shown on the field.
    pub show_draws: bool,
    /// Saved scenario names offered in the scenario menu.
    pub scenarios: Vec<String>,
}

impl Toolbar {
    pub fn new(simulator: SimulatorSettings) -> Self {
        Self {
            script_path: String::new(),
            script_status: ScriptStatus::NoScript,
            simulator,
            recording_s: None,
            has_recording: false,
            show_draws: true,
            scenarios: Vec::new(),
        }
    }

    /// `robot_counts`: robots on the field per team (blue, yellow).
    pub fn view(&self, robot_counts: [usize; 2]) -> Element<'_, ToolbarMessage> {
        let has_script = !self.script_path.is_empty();
        let is_running = self.script_status == ScriptStatus::Running;

        // `tint`: stroked icons take the button's text color; play/stop keep their own.
        let icon_btn = |glyph: Icon, tint: bool, message: Option<ToolbarMessage>| {
            let enabled = message.is_some();
            let color = if tint {
                Some(btn_fg(Btn::Neutral, enabled))
            } else if enabled {
                None
            } else {
                Some(theme::MUTED)
            };
            button(container(icon(glyph, 15.0, color)).center(Length::Fill))
                .width(28)
                .height(26)
                .padding(0)
                .style(btn(Btn::Neutral))
                .on_press_maybe(message)
        };

        let load_btn = icon_btn(Icon::Folder, true, Some(ToolbarMessage::LoadScript));
        // Play/Stop toggle: play when paused/loaded/idle, stop when running
        let toggle_btn = icon_btn(
            if is_running { Icon::Stop } else { Icon::Play },
            false,
            has_script.then_some(ToolbarMessage::ToggleScript),
        );
        let reload_btn = icon_btn(Icon::Reload, true, has_script.then_some(ToolbarMessage::ReloadScript));

        let script_name = if has_script {
            self.script_path.replace('\\', "/").rsplit('/').next().unwrap_or("No script").to_string()
        } else {
            "No script".to_string()
        };

        let status = match self.script_status {
            ScriptStatus::Running => Some(("Running", theme::GREEN, theme::GREEN_DIM)),
            ScriptStatus::Paused => Some(("Paused", theme::ORANGE, theme::ORANGE_DIM)),
            ScriptStatus::Failed => Some(("Failed", theme::RED_FG, theme::RED_DIM)),
            ScriptStatus::NoScript => None,
        };

        // Record toggle: shows the recorded time while recording
        let rec_btn = label_button(
            match self.recording_s {
                Some(secs) => format!("■ REC {secs:.1}s"),
                None => "● REC".to_string(),
            },
            Btn::Rec(self.recording_s.is_some()),
            Some(ToolbarMessage::ToggleRecording),
        );
        // Replay of the last recording, once there is one and nothing is being recorded
        let replay_btn = label_button(
            "REPLAY".to_string(),
            Btn::Neutral,
            (self.has_recording && self.recording_s.is_none()).then_some(ToolbarMessage::PlayRecording),
        );
        let draws_btn = draws_button(self.show_draws, ToolbarMessage::ToggleDraws);

        let mut content = row![
            load_btn,
            toggle_btn,
            reload_btn,
            Space::new().width(4),
            text(script_name)
                .size(12)
                .font(theme::MONO)
                .color(if has_script { theme::TEXT } else { theme::MUTED }),
        ]
        .spacing(4)
        .align_y(Alignment::Center);

        if let Some((label, fg, bg)) = status {
            content = content.push(container(text(label).size(11).color(fg)).padding([2, 8]).style(pill(bg)));
        }

        content = content
            .push(divider())
            .push(rec_btn)
            .push(replay_btn)
            .push(draws_btn)
            .push(Space::new().width(Length::Fill))
            .push(label_button(
                "SIM".to_string(),
                Btn::Toggle(self.simulator.enabled),
                Some(ToolbarMessage::ToggleSimulator),
            ));

        // While the simulator is on: team steppers, scenario menu and speed
        if self.simulator.enabled {
            content = content
                .push(divider())
                .push(team_stepper(0, robot_counts[0]))
                .push(team_stepper(1, robot_counts[1]))
                .push(
                    pick_list(self.scenarios.as_slice(), None::<String>, ToolbarMessage::ScenarioSelected)
                        .placeholder("Load scenario")
                        .on_open(ToolbarMessage::ScenarioMenuOpened)
                        .width(130)
                        .text_size(12),
                )
                .push(
                    pick_list(SIM_SPEEDS, Some(SimSpeed(self.simulator.speed)), ToolbarMessage::SimSpeedSelected)
                        .width(64)
                        .font(theme::MONO)
                        .text_size(12),
                );
        }

        container(content)
            .width(Length::Fill)
            .height(36)
            .padding([0, 8])
            .center_y(36)
            .style(bar(theme::BAR))
            .into()
    }
}

/// Text button of the toolbars (REC, REPLAY, DRAWS, SIM, ...).
pub fn label_button<'a, M: Clone + 'a>(label: String, kind: Btn, message: Option<M>) -> iced::widget::Button<'a, M> {
    button(text(label).size(11).font(theme::SANS_BOLD))
        .height(26)
        .padding([5, 9])
        .style(btn(kind))
        .on_press_maybe(message)
}

/// Show/hide toggle for the Lua drawings, sending `message`: accent while they are shown.
pub fn draws_button<'a, M: Clone + 'a>(shown: bool, message: M) -> iced::widget::Button<'a, M> {
    label_button("DRAWS".to_string(), Btn::Toggle(shown), Some(message))
}

/// `− ●N +` removing or adding a robot of `team`, with its robot count.
fn team_stepper<'a>(team: i32, count: usize) -> Element<'a, ToolbarMessage> {
    const MAX_ROBOTS_PER_TEAM: usize = 16;
    let step = |label: &'static str, message: Option<ToolbarMessage>| {
        button(text(label).size(13).center())
            .width(22)
            .height(24)
            .padding(0)
            .style(btn(Btn::Ghost))
            .on_press_maybe(message)
    };
    container(
        row![
            step("−", (count > 0).then_some(ToolbarMessage::RemoveRobot(team))),
            row![dot(theme::team_color(team), 10.0), text(count).size(12).font(theme::MONO_BOLD)]
                .spacing(5)
                .padding([0, 4])
                .align_y(Alignment::Center),
            step("+", (count < MAX_ROBOTS_PER_TEAM).then_some(ToolbarMessage::AddRobot(team))),
        ]
        .align_y(Alignment::Center),
    )
    .height(26)
    .center_y(26)
    .style(group)
    .into()
}
