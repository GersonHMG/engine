// gui/panels/recording.rs — CSV recording panel

use iced::widget::{button, column, container, row, text, text_input};
use iced::{Element, Length};
use crate::gui::theme;

#[derive(Debug, Clone)]
pub enum RecordingMessage {
    FilenameChanged(String),
    Start,
    Stop,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RecordingStatus {
    Idle,
    Recording,
    Saved,
}

#[derive(Debug, Clone)]
pub struct RecordingPanel {
    pub filename: String,
    pub status: RecordingStatus,
}

impl Default for RecordingPanel {
    fn default() -> Self {
        Self {
            filename: "record.csv".into(),
            status: RecordingStatus::Idle,
        }
    }
}

impl RecordingPanel {
    pub fn view(&self) -> Element<RecordingMessage> {
        let is_recording = self.status == RecordingStatus::Recording;

        let (status_text, status_color) = match self.status {
            RecordingStatus::Idle => ("Idle", theme::MUTED),
            RecordingStatus::Recording => ("Recording...", theme::GREEN),
            RecordingStatus::Saved => ("Saved", theme::MUTED),
        };

        let content = column![
            text("Recording").size(16),
            text_input("record.csv", &self.filename).style(theme::input)
                .on_input(RecordingMessage::FilenameChanged)
                .width(Length::Fill),
            row![
                if is_recording {
                    button(text("Start").center().width(Length::Fill))
                        .width(Length::Fill)
                } else {
                    button(text("Start").center().width(Length::Fill))
                        .on_press(RecordingMessage::Start)
                        .width(Length::Fill)
                },
                if is_recording {
                    button(text("Stop").center().width(Length::Fill))
                        .on_press(RecordingMessage::Stop)
                        .width(Length::Fill)
                        .style(button::danger)
                } else {
                    button(text("Stop").center().width(Length::Fill))
                        .width(Length::Fill)
                        .style(button::danger)
                },
            ]
            .spacing(8),
            text(status_text).size(12).color(status_color),
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
