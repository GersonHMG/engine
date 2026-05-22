// gui/bottom_panel.rs — Toggle buttons + robot selector

use iced::widget::{button, checkbox, column, container, row, text, text_input, Space};
use iced::{Element, Length};

use crate::gui::panels::control::Team;

#[derive(Debug, Clone)]
pub enum BottomPanelMessage {
    SetTrace(bool),
    SetVectors(bool),
    SetManualControl(bool),
    TeamSelected(Team),
    RobotIdChanged(String),
    IncrementRobotId,
    DecrementRobotId,
}

pub struct BottomPanel {
    pub trace_on: bool,
    pub vectors_on: bool,
    pub manual_control_on: bool,
    pub control_robot_id: String,
    pub control_team: Team,
}

impl BottomPanel {
    pub fn new() -> Self {
        Self {
            trace_on: false,
            vectors_on: false,
            manual_control_on: false,
            control_robot_id: "0".to_string(),
            control_team: Team::Blue,
        }
    }

    pub fn view(&self) -> Element<BottomPanelMessage> {
        // Team + ID selector — text_input with stacked ▲/▼ overlaid on the right, like <input type="number">
        let id_val: i32 = self.control_robot_id.parse().unwrap_or(0);

        let spin_up = iced::widget::button(
            text("▲").size(6).align_x(iced::alignment::Horizontal::Center),
        )
        .on_press_maybe(if id_val < 12 { Some(BottomPanelMessage::IncrementRobotId) } else { None })
        .style(|theme: &iced::Theme, status| {
            let mut s = iced::widget::button::secondary(theme, status);
            s.border.radius = 0.0.into();
            s
        })
        .padding([0, 3])
        .width(Length::Fixed(14.0))
        .height(Length::Fixed(11.0));

        let spin_down = iced::widget::button(
            text("▼").size(6).align_x(iced::alignment::Horizontal::Center),
        )
        .on_press_maybe(if id_val > 0 { Some(BottomPanelMessage::DecrementRobotId) } else { None })
        .style(|theme: &iced::Theme, status| {
            let mut s = iced::widget::button::secondary(theme, status);
            s.border.radius = 0.0.into();
            s
        })
        .padding([0, 3])
        .width(Length::Fixed(14.0))
        .height(Length::Fixed(11.0));

        let spin_col = column![spin_up, spin_down].spacing(0);

        // Overlay the spin column on the right side of the text_input
        const INPUT_W: f32 = 54.0;
        let number_input = iced::widget::stack![
            text_input("0", &self.control_robot_id)
                .on_input(BottomPanelMessage::RobotIdChanged)
                .size(10)
                .width(Length::Fixed(INPUT_W)),
            container(spin_col)
                .width(Length::Fixed(INPUT_W))
                .height(Length::Fill)
                .align_x(iced::alignment::Horizontal::Right)
                .align_y(iced::alignment::Vertical::Center),
        ];

        let team_box = |team: Team| {
            let selected = self.control_team == team;
            let (r, g, b) = match team {
                Team::Blue => (0.2, 0.55, 1.0),
                Team::Yellow => (1.0, 0.85, 0.2),
            };
            let bg = if selected {
                iced::Color::from_rgb(r, g, b)
            } else {
                iced::Color::from_rgba(r, g, b, 0.35)
            };
            let border_color = if selected {
                iced::Color::WHITE
            } else {
                iced::Color::from_rgb(0.35, 0.35, 0.35)
            };

            button(
                Space::new()
                    .width(Length::Fixed(14.0))
                    .height(Length::Fixed(14.0)),
            )
            .on_press(BottomPanelMessage::TeamSelected(team))
            .style(move |theme: &iced::Theme, status| {
                let mut s = iced::widget::button::secondary(theme, status);
                s.background = Some(iced::Background::Color(bg));
                s.border.color = border_color;
                s.border.width = if selected { 1.5 } else { 1.0 };
                s.border.radius = 2.0.into();
                s
            })
            .padding(0)
            .width(Length::Fixed(18.0))
            .height(Length::Fixed(18.0))
        };

        let selector = row![
            text("Team").size(10),
            row![team_box(Team::Blue), team_box(Team::Yellow)]
                .spacing(4)
                .align_y(iced::Alignment::Center),
            text("ID").size(10),
            number_input,
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center);

        let trace_check = checkbox(self.trace_on)
            .label("Trace")
            .on_toggle(BottomPanelMessage::SetTrace)
            .size(14)
            .text_size(10);

        let vectors_check = checkbox(self.vectors_on)
            .label("Vectors")
            .on_toggle(BottomPanelMessage::SetVectors)
            .size(14)
            .text_size(10);

        let manual_check = checkbox(self.manual_control_on)
            .label("Manual Control")
            .on_toggle(BottomPanelMessage::SetManualControl)
            .size(14)
            .text_size(10);

        let content = row![selector, trace_check, vectors_check, manual_check,]
            .spacing(16)
            .padding(8)
            .align_y(iced::Alignment::Center);

        container(content)
            .width(Length::Fill)
            .height(Length::Fixed(52.0))
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
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}
