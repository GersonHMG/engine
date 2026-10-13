// gui/panels/control.rs — Manual control panel

use iced::widget::{column, container, pick_list, row, text, text_input};
use iced::{Element, Length};
use crate::gui::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlMode {
    Xbox,
    Keyboard,
}

impl std::fmt::Display for ControlMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ControlMode::Xbox => write!(f, "Xbox"),
            ControlMode::Keyboard => write!(f, "Keyboard"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Team {
    Blue,
    Yellow,
}

impl std::fmt::Display for Team {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Team::Blue => write!(f, "Blue"),
            Team::Yellow => write!(f, "Yellow"),
        }
    }
}

impl Team {
    pub fn to_id(self) -> i32 {
        match self {
            Team::Blue => 0,
            Team::Yellow => 1,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ControlMessage {
    ModeSelected(ControlMode),
    ScaleVxChanged(String),
    ScaleVyChanged(String),
    ScaleWChanged(String),
}

#[derive(Debug, Clone)]
pub struct ControlPanel {
    pub mode: ControlMode,
    pub active: bool,
    pub team: Team,
    pub robot_id: String,
    pub scale_vx: String,
    pub scale_vy: String,
    pub scale_w: String,
    pub vis_velocities: bool,
}

impl Default for ControlPanel {
    fn default() -> Self {
        Self {
            mode: ControlMode::Keyboard,
            active: false,
            team: Team::Blue,
            robot_id: "0".into(),
            scale_vx: "1.0".into(),
            scale_vy: "1.0".into(),
            scale_w: "1.0".into(),
            vis_velocities: false,
        }
    }
}

impl ControlPanel {
    pub fn robot_id_parsed(&self) -> i32 {
        self.robot_id.parse().unwrap_or(0)
    }

    pub fn scale_vx_parsed(&self) -> f64 {
        self.scale_vx.parse().unwrap_or(1.0)
    }

    pub fn scale_vy_parsed(&self) -> f64 {
        self.scale_vy.parse().unwrap_or(1.0)
    }

    pub fn scale_w_parsed(&self) -> f64 {
        self.scale_w.parse().unwrap_or(1.0)
    }

    pub fn view(&self) -> Element<ControlMessage> {
        let robot_info = format!("Robot {} · {}", self.robot_id, self.team);

        let content = column![
            text("Manual Control").size(16),
            text(robot_info).size(12).color(theme::TEXT_2),
            row![
                text("Mode").width(Length::Fixed(90.0)),
                pick_list(
                    &[ControlMode::Keyboard, ControlMode::Xbox][..],
                    Some(self.mode),
                    ControlMessage::ModeSelected,
                )
                .width(Length::Fill),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
            row![
                text("Scale Vx").width(Length::Fixed(90.0)),
                text_input("1.0", &self.scale_vx).style(theme::input)
                    .on_input(ControlMessage::ScaleVxChanged)
                    .width(Length::Fixed(60.0)),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
            row![
                text("Scale Vy").width(Length::Fixed(90.0)),
                text_input("1.0", &self.scale_vy).style(theme::input)
                    .on_input(ControlMessage::ScaleVyChanged)
                    .width(Length::Fixed(60.0)),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
            row![
                text("Scale ω").width(Length::Fixed(90.0)),
                text_input("1.0", &self.scale_w).style(theme::input)
                    .on_input(ControlMessage::ScaleWChanged)
                    .width(Length::Fixed(60.0)),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
            text("Keys: W/S Vx, A/D Vy, Q/E ω, K kick")
                .size(11)
                .color(theme::MUTED),
        ]
        .spacing(8)
        .padding(12);

        container(content)
            .width(Length::Fixed(280.0))
            .style(panel_style)
            .into()
    }
}

fn panel_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(theme::PANEL)),
        text_color: Some(theme::TEXT),
        border: iced::Border {
            color: theme::DIVIDER,
            width: 1.0,
            radius: 6.0.into(),
        },
        ..Default::default()
    }
}
