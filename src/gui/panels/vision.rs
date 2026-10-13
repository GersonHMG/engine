// gui/panels/vision.rs — Vision connection settings panel

use iced::widget::{button, column, container, row, text, text_input};
use iced::{Element, Length};
use crate::gui::theme;

#[derive(Debug, Clone)]
pub enum VisionMessage {
    IpChanged(String),
    PortChanged(String),
    Reconnect,
}

#[derive(Debug, Clone)]
pub struct VisionPanel {
    pub ip: String,
    pub port: String,
    pub connected: bool,
    pub pps: u32,
}

impl Default for VisionPanel {
    fn default() -> Self {
        Self {
            ip: "224.5.23.2".into(),
            port: "10020".into(),
            connected: false,
            pps: 0,
        }
    }
}

impl VisionPanel {
    pub fn view(&self) -> Element<VisionMessage> {
        let status_text = if self.connected {
            format!("Connected ({} PPS)", self.pps)
        } else {
            "Disconnected".into()
        };

        let status_color = if self.connected {
            theme::GREEN
        } else {
            theme::RED_FG
        };

        let content = column![
            text("Vision Connection").size(16),
            row![
                text("IP").width(Length::Fixed(80.0)),
                text_input("224.5.23.2", &self.ip).style(theme::input)
                    .on_input(VisionMessage::IpChanged)
                    .width(Length::Fill),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
            row![
                text("Port").width(Length::Fixed(80.0)),
                text_input("10020", &self.port).style(theme::input)
                    .on_input(VisionMessage::PortChanged)
                    .width(Length::Fill),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
            row![
                text("Status").width(Length::Fixed(80.0)),
                text(status_text).color(status_color),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
            button(text("Reconnect").center().width(Length::Fill))
                .on_press(VisionMessage::Reconnect)
                .width(Length::Fill),
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
