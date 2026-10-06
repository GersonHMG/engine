// gui/teleport_menu.rs — Right-click menu on the field: move the ball or the
// selected robot to the clicked point, or add a robot there (simulator or
// grSim).

use iced::widget::{button, column, container, pin, text};
use iced::{Color, Element, Length, Point};

const MENU_WIDTH: f32 = 190.0;

#[derive(Debug, Clone)]
pub enum TeleportMenuMessage {
    Ball,
    SelectedRobot,
    /// Add a robot of this team (0 = blue, 1 = yellow).
    AddRobot { team: i32 },
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
    /// `selected`: the robot to teleport, if any (id, team).
    /// `can_add`: whether each team (blue, yellow) has a free robot id.
    pub fn view<'a>(&self, selected: Option<(u32, i32)>, can_add: [bool; 2]) -> Element<'a, TeleportMenuMessage> {
        let item = |label: String, message: Option<TeleportMenuMessage>| {
            button(text(label).size(12))
                .on_press_maybe(message)
                .style(button::text)
                .width(Length::Fill)
        };

        let teleport_label = match selected {
            Some((id, team)) => format!("Teleport {} {id} here", team_name(team)),
            None => "Teleport robot here (click one first)".to_string(),
        };

        let items = column![
            item("Move ball here".to_string(), Some(TeleportMenuMessage::Ball)),
            item(teleport_label, selected.map(|_| TeleportMenuMessage::SelectedRobot)),
            item(
                "Add blue robot here".to_string(),
                can_add[0].then_some(TeleportMenuMessage::AddRobot { team: 0 }),
            ),
            item(
                "Add yellow robot here".to_string(),
                can_add[1].then_some(TeleportMenuMessage::AddRobot { team: 1 }),
            ),
        ];

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

fn team_name(team: i32) -> &'static str {
    if team == 0 {
        "blue"
    } else {
        "yellow"
    }
}
