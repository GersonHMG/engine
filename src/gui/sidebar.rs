// gui/sidebar.rs — Sidebar with icon buttons to open panels

use super::theme::{bar, btn, btn_fg, icon, tooltip_box, Btn, Icon, DIVIDER, SIDEBAR};
use iced::widget::{button, column, container, text, tooltip, Space};
use iced::{Alignment, Element, Length};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SidebarPanel {
    Vision,
    Radio,
    Kalman,
    Recording,
    Control,
    Charts,
    LuaConsole,
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

    /// `open`: panels whose window is open; `replay_mode` disables the panels.
    pub fn view(&self, open: &[SidebarPanel], replay_mode: bool, lua_console_open: bool) -> Element<'_, SidebarMessage> {
        let item = |glyph: Icon, label: &'static str, kind: Btn, message: Option<SidebarMessage>| -> Element<'_, SidebarMessage> {
            let enabled = message.is_some();
            let btn = button(container(icon(glyph, 16.0, Some(btn_fg(kind, enabled)))).center(Length::Fill))
                .width(32)
                .height(32)
                .padding(0)
                .style(btn(kind))
                .on_press_maybe(message);
            tooltip(btn, container(text(label).size(12)).padding([4, 8]).style(tooltip_box), tooltip::Position::Right)
                .gap(6)
                .into()
        };
        let panel = |glyph: Icon, label: &'static str, panel: SidebarPanel| {
            let message = (!replay_mode).then_some(SidebarMessage::TogglePanel(panel));
            item(glyph, label, Btn::Side(open.contains(&panel)), message)
        };

        let content = column![
            panel(Icon::Eye, "Vision", SidebarPanel::Vision),
            panel(Icon::Radio, "Radio", SidebarPanel::Radio),
            panel(Icon::Filter, "Kalman filter", SidebarPanel::Kalman),
            panel(Icon::Record, "Recording", SidebarPanel::Recording),
            panel(Icon::Gamepad, "Manual control", SidebarPanel::Control),
            panel(Icon::Chart, "Charts", SidebarPanel::Charts),
            container(Space::new().width(22).height(1)).style(bar(DIVIDER)),
            item(
                Icon::Terminal,
                "Lua console",
                Btn::Side(lua_console_open),
                (!replay_mode).then_some(SidebarMessage::TogglePanel(SidebarPanel::LuaConsole)),
            ),
            item(
                Icon::Replay,
                if replay_mode { "Exit replay mode" } else { "Replay mode" },
                Btn::Replay(replay_mode),
                Some(SidebarMessage::ToggleReplayMode),
            ),
        ]
        .spacing(4)
        .align_x(Alignment::Center);

        container(content)
            .width(44)
            .height(Length::Fill)
            .padding([6, 0])
            .style(bar(SIDEBAR))
            .into()
    }
}

