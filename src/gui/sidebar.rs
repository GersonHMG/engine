// gui/sidebar.rs — Sidebar with icon buttons to open panels

use iced::widget::{button, column, container, text};
use iced::{Element, Length};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SidebarPanel {
    Vision,
    Radio,
    Kalman,
    Recording,
    Control,
    Charts,
}

#[derive(Debug, Clone)]
pub enum SidebarMessage {
    TogglePanel(SidebarPanel),
    ToggleReplayMode,
}

pub struct Sidebar {
    pub active_panel: Option<SidebarPanel>,
}

impl Sidebar {
    pub fn new() -> Self {
        Self {
            active_panel: None,
        }
    }

    pub fn toggle(&mut self, panel: SidebarPanel) {
        if self.active_panel == Some(panel) {
            self.active_panel = None;
        } else {
            self.active_panel = Some(panel);
        }
    }

    pub fn view(&self, replay_mode: bool) -> Element<SidebarMessage> {
        let make_btn = |label: &'static str, panel: SidebarPanel, is_active: bool, enabled: bool| -> Element<SidebarMessage> {
            let btn = button(
                text(label)
                    .size(14)
                    .center()
                    .width(Length::Fill),
            )
            .width(Length::Fixed(36.0))
            .height(Length::Fixed(36.0))
            .style(if is_active {
                button::primary
            } else {
                button::secondary
            });

            if enabled {
                btn.on_press(SidebarMessage::TogglePanel(panel)).into()
            } else {
                btn.into()
            }
        };

        let ap = self.active_panel;
        let replay_btn = if replay_mode {
            button(text("R").size(14).center().width(Length::Fill))
                .on_press(SidebarMessage::ToggleReplayMode)
                .width(Length::Fixed(36.0))
                .height(Length::Fixed(36.0))
                .style(button::danger)
        } else {
            button(text("R").size(14).center().width(Length::Fill))
                .on_press(SidebarMessage::ToggleReplayMode)
                .width(Length::Fixed(36.0))
                .height(Length::Fixed(36.0))
                .style(button::success)
        };

        let content = column![
            make_btn("👁", SidebarPanel::Vision, ap == Some(SidebarPanel::Vision), !replay_mode),
            make_btn("📡", SidebarPanel::Radio, ap == Some(SidebarPanel::Radio), !replay_mode),
            make_btn("📈", SidebarPanel::Kalman, ap == Some(SidebarPanel::Kalman), !replay_mode),
            make_btn("⏺", SidebarPanel::Recording, ap == Some(SidebarPanel::Recording), !replay_mode),
            make_btn("🎮", SidebarPanel::Control, ap == Some(SidebarPanel::Control), !replay_mode),
            make_btn("📊", SidebarPanel::Charts, ap == Some(SidebarPanel::Charts), !replay_mode),
            replay_btn,
        ]
        .spacing(4)
        .padding(4);

        container(content)
            .style(sidebar_style)
            .into()
    }
}

fn sidebar_style(theme: &iced::Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(iced::Background::Color(palette.background.weak.color)),
        border: iced::Border {
            color: palette.background.strong.color,
            width: 0.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}
