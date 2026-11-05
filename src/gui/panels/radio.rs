// gui/panels/radio.rs — Radio configuration panel

use iced::widget::column;
use iced::Element;
use super::pipeline::{field, input, switch};

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
    /// Settings shown on the Radio card of the pipeline window.
    pub fn body(&self) -> Element<'_, RadioMessage> {
        column![
            switch("Serial radio (off = grSim)", self.use_radio, RadioMessage::UseRadioToggled),
            field("SERIAL PORT", input("/dev/ttyUSB0", &self.port_name, RadioMessage::PortNameChanged)),
            field("BAUD RATE", input("115200", &self.baud_rate, RadioMessage::BaudRateChanged)),
        ]
        .spacing(10)
        .into()
    }
}
