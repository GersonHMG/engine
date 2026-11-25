// gui/panels/radio.rs — Radio configuration panel
//
// The serial port can be typed or picked from the ports found on this
// computer (Scan lists them again, e.g. after plugging the radio in).

use iced::widget::{button, column, pick_list, row, text};
use iced::{Alignment, Element, Length};
use std::fmt;

use super::pipeline::{field, input, switch};
use crate::gui::theme::{self, btn, Btn};

#[derive(Debug, Clone)]
pub enum RadioMessage {
    PortNameChanged(String),
    BaudRateChanged(String),
    UseRadioToggled(bool),
    /// Lists the serial ports again.
    ScanPorts,
    PortSelected(SerialPortChoice),
    Update,
}

/// A serial port found on this computer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerialPortChoice {
    /// What the radio opens, e.g. `COM3` or `/dev/ttyUSB0`.
    pub name: String,
    /// What it is, when the system says (e.g. the USB adapter's name).
    pub description: Option<String>,
}

impl fmt::Display for SerialPortChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.description {
            Some(d) => write!(f, "{} · {d}", self.name),
            None => write!(f, "{}", self.name),
        }
    }
}

/// The serial ports on this computer, by name.
pub fn scan_serial_ports() -> Vec<SerialPortChoice> {
    let mut ports: Vec<SerialPortChoice> = serialport::available_ports()
        .unwrap_or_default()
        .into_iter()
        .map(|p| {
            let description = match p.port_type {
                serialport::SerialPortType::UsbPort(usb) => usb
                    .product
                    .or(usb.manufacturer)
                    .or_else(|| Some(format!("USB {:04x}:{:04x}", usb.vid, usb.pid))),
                serialport::SerialPortType::BluetoothPort => Some("Bluetooth".to_string()),
                _ => None,
            };
            SerialPortChoice { name: p.port_name, description }
        })
        .collect();
    ports.sort_by(|a, b| a.name.cmp(&b.name));
    ports.dedup_by(|a, b| a.name == b.name);
    ports
}

#[derive(Debug, Clone)]
pub struct RadioPanel {
    pub port_name: String,
    pub baud_rate: String,
    pub use_radio: bool,
    /// Ports found by the last scan; None until the first one.
    pub ports: Option<Vec<SerialPortChoice>>,
}

impl Default for RadioPanel {
    fn default() -> Self {
        Self {
            port_name: "/dev/ttyUSB0".into(),
            baud_rate: "115200".into(),
            use_radio: false,
            ports: None,
        }
    }
}

impl RadioPanel {
    /// Lists the serial ports again.
    pub fn scan_ports(&mut self) {
        self.ports = Some(scan_serial_ports());
    }

    /// Settings shown on the Radio stage of the pipeline window.
    pub fn body(&self) -> Element<'_, RadioMessage> {
        let scan = button(text("Scan").size(11))
            .padding([5, 10])
            .style(btn(Btn::Neutral))
            .on_press(RadioMessage::ScanPorts);
        let port_row = row![
            container_fill(input("/dev/ttyUSB0 or COM3", &self.port_name, RadioMessage::PortNameChanged)),
            scan,
        ]
        .spacing(6)
        .align_y(Alignment::Center);

        let found: Element<'_, RadioMessage> = match &self.ports {
            Some(ports) if !ports.is_empty() => {
                let selected = ports.iter().find(|p| p.name == self.port_name).cloned();
                pick_list(ports.as_slice(), selected, RadioMessage::PortSelected)
                    .placeholder(format!("{} port{} found — pick one", ports.len(), if ports.len() == 1 { "" } else { "s" }))
                    .text_size(12)
                    .width(Length::Fill)
                    .into()
            }
            Some(_) => text("No serial ports found").size(11).color(theme::MUTED).into(),
            None => text("Scan lists the serial ports on this computer").size(11).color(theme::MUTED).into(),
        };

        column![
            switch("Serial radio (off = grSim)", self.use_radio, RadioMessage::UseRadioToggled),
            field("SERIAL PORT", column![port_row, found].spacing(6)),
            field("BAUD RATE", input("115200", &self.baud_rate, RadioMessage::BaudRateChanged)),
        ]
        .spacing(10)
        .into()
    }
}

fn container_fill<'a>(e: Element<'a, RadioMessage>) -> Element<'a, RadioMessage> {
    iced::widget::container(e).width(Length::Fill).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ports_read_with_their_description() {
        let port = SerialPortChoice { name: "COM3".into(), description: Some("CP2102 USB to UART".into()) };
        assert_eq!(port.to_string(), "COM3 · CP2102 USB to UART");
        let bare = SerialPortChoice { name: "/dev/ttyS0".into(), description: None };
        assert_eq!(bare.to_string(), "/dev/ttyS0");
    }

    #[test]
    fn scanning_does_not_fail_without_ports() {
        let mut panel = RadioPanel::default();
        panel.scan_ports();
        assert!(panel.ports.is_some());
    }
}
