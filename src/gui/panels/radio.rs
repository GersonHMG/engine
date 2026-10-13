// gui/panels/radio.rs — Radio configuration panel

use iced::widget::{button, column, container, row, text, text_input, toggler};
use iced::{Element, Length};
use crate::gui::theme;

#[derive(Debug, Clone)]
pub enum RadioMessage {
    PortNameChanged(String),
    BaudRateChanged(String),
    UseRadioToggled(bool),
    Update,
}

#[derive(Debug, Clone)]
pub struct RadioPanel {
    pub port_name: String,
    pub baud_rate: String,
    pub use_radio: bool,
}

impl Default for RadioPanel {
    fn default() -> Self {
        Self {
            port_name: "/dev/ttyUSB0".into(),
            baud_rate: "115200".into(),
            use_radio: false,
        }
    }
}

impl RadioPanel {
    pub fn view(&self) -> Element<RadioMessage> {
        let content = column![
            text("Radio Configuration").size(16),
            row![
                text("Port").width(Length::Fixed(80.0)),
                text_input("/dev/ttyUSB0", &self.port_name).style(theme::input)
                    .on_input(RadioMessage::PortNameChanged)
                    .width(Length::Fill),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
            row![
                text("Baud Rate").width(Length::Fixed(80.0)),
                text_input("115200", &self.baud_rate).style(theme::input)
                    .on_input(RadioMessage::BaudRateChanged)
                    .width(Length::Fill),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
            row![
                text("Use Radio").width(Length::Fixed(80.0)),
                toggler(self.use_radio)
                    .on_toggle(RadioMessage::UseRadioToggled),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
            button(text("Update Radio").center().width(Length::Fill))
                .on_press(RadioMessage::Update)
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
