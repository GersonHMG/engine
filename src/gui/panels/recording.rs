// gui/panels/recording.rs — CSV recording panel

use iced::widget::{button, column, container, row, text, text_input};
use iced::{Element, Length};

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
            RecordingStatus::Idle => ("Idle", iced::Color::from_rgb(0.5, 0.5, 0.5)),
            RecordingStatus::Recording => ("Recording...", iced::Color::from_rgb(0.2, 0.8, 0.2)),
            RecordingStatus::Saved => ("Saved", iced::Color::from_rgb(0.5, 0.5, 0.5)),
        };

        let content = column![
            text("Recording").size(16),
            text_input("record.csv", &self.filename)
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

fn panel_style(theme: &iced::Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(iced::Background::Color(palette.background.weak.color)),
        border: iced::Border {
            color: palette.background.strong.color,
            width: 1.0,
            radius: 6.0.into(),
        },
        ..Default::default()
    }
}
