// gui/teleport_menu.rs — Right-click menu on the field: move the ball or a
// robot to the clicked point (simulator or grSim).

use super::robot::RobotData;
use iced::widget::{button, column, container, pin, text};
use iced::{Color, Element, Length, Point};

const MENU_WIDTH: f32 = 170.0;

#[derive(Debug, Clone)]
pub enum TeleportMenuMessage {
    Ball,
    Robot { id: u32, team: i32 },
}

/// An open menu and the point it was opened at.
#[derive(Debug, Clone, Copy)]
pub struct TeleportMenu {
    /// Click position inside the field canvas, px.
    pub screen: Point,
    /// Click position on the field, m.
    pub field: (f64, f64),
}

impl TeleportMenu {
    /// The menu at the click position: the ball, then every visible robot.
    pub fn view<'a>(&self, blue: &[RobotData], yellow: &[RobotData]) -> Element<'a, TeleportMenuMessage> {
        let item = |label: String, message: TeleportMenuMessage| {
            button(text(label).size(12))
                .on_press(message)
                .style(button::text)
                .width(Length::Fill)
        };

        let mut items = column![item("Move ball here".to_string(), TeleportMenuMessage::Ball)];
        for (team, name, robots) in [(0, "blue", blue), (1, "yellow", yellow)] {
            let mut ids: Vec<u32> = robots.iter().map(|r| r.id).collect();
            ids.sort_unstable();
            for id in ids {
                items = items.push(item(format!("Move {name} {id} here"), TeleportMenuMessage::Robot { id, team }));
            }
        }

        let menu = container(items.width(MENU_WIDTH)).padding(4).style(|theme: &iced::Theme| {
            let palette = theme.extended_palette();
            container::Style {
                background: Some(iced::Background::Color(palette.background.weak.color)),
                border: iced::Border {
                    color: Color::from_rgb(0.4, 0.4, 0.4),
                    width: 1.0,
                    radius: 4.0.into(),
                },
                ..Default::default()
            }
        });

        pin(menu).position(self.screen).into()
    }
}
