// gui/panels/vision.rs — Vision connection settings panel

use iced::widget::column;
use iced::Element;
use super::pipeline::{field, input};

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
    /// Settings shown on the Vision card of the pipeline window.
    pub fn body(&self) -> Element<'_, VisionMessage> {
        column![
            field("MULTICAST ADDRESS", input("224.5.23.2", &self.ip, VisionMessage::IpChanged)),
            field("PORT", input("10020", &self.port, VisionMessage::PortChanged)),
        ]
        .spacing(10)
        .into()
    }
}
