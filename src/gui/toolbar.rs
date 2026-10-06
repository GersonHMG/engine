// gui/toolbar.rs — Script toolbar (load/play/reload + simulator switch + PPS sparkline + filename)

use super::types::SimulatorSettings;
use iced::widget::canvas::{self, Frame, Geometry, Path, Stroke};
use iced::widget::{button, container, pick_list, row, text, Canvas};
use iced::{mouse, Color, Element, Length, Point, Rectangle, Size, Theme};
use std::fmt;

const PPS_HISTORY: usize = 60;

/// Speeds offered for the simulator; 0 = as fast as possible.
const SIM_SPEEDS: [SimSpeed; 5] = [SimSpeed(1.0), SimSpeed(2.0), SimSpeed(5.0), SimSpeed(10.0), SimSpeed(0.0)];

#[derive(Debug, Clone)]
pub enum ToolbarMessage {
    LoadScript,
    ToggleScript,
    ReloadScript,
    ToggleSimulator,
    SimSpeedSelected(SimSpeed),
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
    pub pps: u32,
    pub pps_history: Vec<u32>,
    pub simulator: SimulatorSettings,
}

impl Toolbar {
    pub fn new(simulator: SimulatorSettings) -> Self {
        Self {
            script_path: String::new(),
            script_status: ScriptStatus::NoScript,
            pps: 0,
            pps_history: vec![0; PPS_HISTORY],
            simulator,
        }
    }

    pub fn push_pps(&mut self, pps: u32) {
        self.pps = pps;
        self.pps_history.remove(0);
        self.pps_history.push(pps);
    }

    pub fn view(&self) -> Element<ToolbarMessage> {
        let has_script = !self.script_path.is_empty();
        let is_running = self.script_status == ScriptStatus::Running;

        let load_btn = button(text("📂").size(14))
            .on_press(ToolbarMessage::LoadScript)
            .style(button::secondary);

        // Play/Stop toggle: ▶ when paused/loaded/idle, ⏹ when running
        let toggle_btn = if has_script {
            if is_running {
                button(text("⏹").size(14))
                    .on_press(ToolbarMessage::ToggleScript)
                    .style(button::danger)
            } else {
                button(text("▶").size(14))
                    .on_press(ToolbarMessage::ToggleScript)
                    .style(button::success)
            }
        } else {
            button(text("▶").size(14)).style(button::secondary)
        };

        // Reload button
        let reload_btn = if has_script {
            button(text("🔄").size(14))
                .on_press(ToolbarMessage::ReloadScript)
                .style(button::secondary)
        } else {
            button(text("🔄").size(14)).style(button::secondary)
        };

        let script_name = if self.script_path.is_empty() {
            "No script".to_string()
        } else {
            self.script_path
                .replace('\\', "/")
                .rsplit('/')
                .next()
                .unwrap_or("No script")
                .to_string()
        };

        let (status_text, status_color) = match self.script_status {
            ScriptStatus::Running => ("Running", Color::from_rgb(0.2, 0.85, 0.25)),
            ScriptStatus::Paused => ("Paused", Color::from_rgb(0.95, 0.55, 0.15)),
            ScriptStatus::NoScript => ("", Color::from_rgb(0.6, 0.6, 0.6)),
            ScriptStatus::Failed => ("Failed", Color::from_rgb(0.9, 0.2, 0.2)),
        };

        let pps_text = format!("{} PPS", self.pps);

        // PPS sparkline (small inline canvas)
        let pps_sparkline = Canvas::new(PpsSparkline {
            data: &self.pps_history,
        })
        .width(Length::Fixed(80.0))
        .height(Length::Fixed(20.0));

        // Simulator switch, plus its speed menu while it is on
        let sim_btn = button(text("SIM").size(12))
            .on_press(ToolbarMessage::ToggleSimulator)
            .style(if self.simulator.enabled { button::success } else { button::secondary });

        let mut content = row![
            load_btn,
            toggle_btn,
            reload_btn,
            text("|").size(14).color(Color::from_rgb(0.3, 0.3, 0.3)),
            text(script_name).size(12).color(Color::from_rgb(0.6, 0.6, 0.6)),
            text(status_text).size(12).color(status_color),
            iced::widget::Space::new().width(Length::Fill),
            sim_btn,
        ]
        .spacing(6)
        .padding(4)
        .align_y(iced::Alignment::Center);

        if self.simulator.enabled {
            content = content.push(
                pick_list(SIM_SPEEDS, Some(SimSpeed(self.simulator.speed)), ToolbarMessage::SimSpeedSelected)
                    .text_size(12),
            );
        }

        let content = content
            .push(text(pps_text).size(12).color(Color::from_rgb(0.6, 0.6, 0.6)))
            .push(pps_sparkline);

        container(content)
            .width(Length::Fill)
            .height(Length::Fixed(36.0))
            .style(toolbar_style)
            .into()
    }
}

fn toolbar_style(theme: &iced::Theme) -> container::Style {
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
}

// --- PPS Sparkline Canvas ---
struct PpsSparkline<'a> {
    data: &'a [u32],
}

impl<'a> canvas::Program<ToolbarMessage> for PpsSparkline<'a> {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let w = bounds.width;
        let h = bounds.height;

        // Background
        frame.fill_rectangle(
            Point::ORIGIN,
            Size::new(w, h),
            Color::from_rgb(0.1, 0.1, 0.15),
        );

        if self.data.is_empty() {
            return vec![frame.into_geometry()];
        }

        let max_pps = self.data.iter().copied().max().unwrap_or(1).max(1) as f32;
        let n = self.data.len();

        if n >= 2 {
            let path = Path::new(|b| {
                for (i, v) in self.data.iter().enumerate() {
                    let px = (i as f32 / (n - 1) as f32) * w;
                    let py = h - (*v as f32 / max_pps) * h * 0.9;
                    if i == 0 {
                        b.move_to(Point::new(px, py));
                    } else {
                        b.line_to(Point::new(px, py));
                    }
                }
            });
            frame.stroke(
                &path,
                Stroke::default()
                    .with_color(Color::from_rgb(0.3, 0.8, 0.4))
                    .with_width(1.0),
            );
        }

        vec![frame.into_geometry()]
    }
}
