// gui/panels/kalman.rs — Kalman filter configuration panel

use iced::widget::{button, column, container, row, text, text_input, toggler};
use iced::{Element, Length};

#[derive(Debug, Clone)]
pub enum KalmanMessage {
    EnabledToggled(bool),
    ProcessNoisePChanged(String),
    ProcessNoiseVChanged(String),
    MeasurementNoiseChanged(String),
    Update,
}

#[derive(Debug, Clone)]
pub struct KalmanPanel {
    pub enabled: bool,
    pub process_noise_p: String,
    pub process_noise_v: String,
    pub measurement_noise: String,
}

impl Default for KalmanPanel {
    fn default() -> Self {
        Self {
            enabled: true,
            process_noise_p: "0.0000001".into(),
            process_noise_v: "0.0001".into(),
            measurement_noise: "0.000001".into(),
        }
    }
}

impl KalmanPanel {
    pub fn view(&self) -> Element<KalmanMessage> {
        let content = column![
            text("Kalman Filter").size(16),
            row![
                text("Enabled").width(Length::Fixed(100.0)),
                toggler(self.enabled)
                    .on_toggle(KalmanMessage::EnabledToggled),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
            row![
                text("Proc. Noise (P)").width(Length::Fixed(100.0)),
                text_input("0.0000001", &self.process_noise_p)
                    .on_input(KalmanMessage::ProcessNoisePChanged)
                    .width(Length::Fill),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
            row![
                text("Proc. Noise (V)").width(Length::Fixed(100.0)),
                text_input("0.0001", &self.process_noise_v)
                    .on_input(KalmanMessage::ProcessNoiseVChanged)
                    .width(Length::Fill),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
            row![
                text("Meas. Noise").width(Length::Fixed(100.0)),
                text_input("0.000001", &self.measurement_noise)
                    .on_input(KalmanMessage::MeasurementNoiseChanged)
                    .width(Length::Fill),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
            button(text("Update KF").center().width(Length::Fill))
                .on_press(KalmanMessage::Update)
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
