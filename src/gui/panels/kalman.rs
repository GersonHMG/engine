// gui/panels/kalman.rs — Kalman filter configuration panel

use iced::widget::column;
use iced::Element;
use super::pipeline::{field, input, switch};

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
    /// Settings shown on the Kalman card of the pipeline window.
    pub fn body(&self) -> Element<'_, KalmanMessage> {
        column![
            switch("Filter enabled", self.enabled, KalmanMessage::EnabledToggled),
            field("PROCESS NOISE · POSITION", input("0.0000001", &self.process_noise_p, KalmanMessage::ProcessNoisePChanged)),
            field("PROCESS NOISE · VELOCITY", input("0.0001", &self.process_noise_v, KalmanMessage::ProcessNoiseVChanged)),
            field("MEASUREMENT NOISE", input("0.000001", &self.measurement_noise, KalmanMessage::MeasurementNoiseChanged)),
        ]
        .spacing(10)
        .into()
    }
}
