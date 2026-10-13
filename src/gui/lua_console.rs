// gui/lua_console.rs — Lua console panel UI: logs plus a command line
// (simulator commands, see `sim help`)

use super::theme::{self, btn, Btn};
use iced::widget::{button, column, container, row, scrollable, text, text_input, Id, MouseArea, Space};
use iced::{mouse, Alignment, Element, Length};
use std::collections::VecDeque;

const DEFAULT_MAX_LINES: usize = 100;
const DEFAULT_PANEL_HEIGHT_PX: f32 = 140.0;
pub const MIN_PANEL_HEIGHT_PX: f32 = 90.0;
pub const MAX_PANEL_HEIGHT_PX: f32 = 320.0;
pub const RESIZE_EDGE_PX: f32 = 6.0;
const MIN_SCROLLBAR_THUMB_PX: f32 = 24.0;
const APPROX_LINE_HEIGHT_PX: f32 = 14.0;

pub struct LuaConsolePanel {
    pub open: bool,
    logs: VecDeque<String>,
    max_lines: usize,
    scroll_id: Id,
    height: f32,
    /// Command being typed.
    input: String,
}

impl LuaConsolePanel {
    pub fn new() -> Self {
        let max_lines = max_lines_for_min_thumb(DEFAULT_PANEL_HEIGHT_PX);
        Self {
            open: false,
            logs: VecDeque::new(),
            max_lines: DEFAULT_MAX_LINES.min(max_lines),
            scroll_id: Id::unique(),
            height: DEFAULT_PANEL_HEIGHT_PX,
            input: String::new(),
        }
    }

    pub fn set_input(&mut self, input: String) {
        self.input = input;
    }

    /// The typed command, trimmed, clearing the input; `None` if empty.
    pub fn take_input(&mut self) -> Option<String> {
        let line = std::mem::take(&mut self.input).trim().to_string();
        (!line.is_empty()).then_some(line)
    }

    pub fn push_line(&mut self, line: String) -> bool {
        if line.trim().is_empty() {
            return false;
        }

        self.logs.push_back(line);
        if self.logs.len() > self.max_lines {
            let overflow = self.logs.len() - self.max_lines;
            for _ in 0..overflow {
                self.logs.pop_front();
            }
        }

        true
    }

    pub fn scroll_id(&self) -> Id {
        self.scroll_id.clone()
    }

    pub fn height(&self) -> f32 {
        self.height
    }

    pub fn set_height(&mut self, height: f32) {
        let clamped = height.clamp(MIN_PANEL_HEIGHT_PX, MAX_PANEL_HEIGHT_PX);
        self.height = clamped;
        let max_lines = max_lines_for_min_thumb(clamped);
        self.max_lines = DEFAULT_MAX_LINES.min(max_lines);
        while self.logs.len() > self.max_lines {
            self.logs.pop_front();
        }
    }

    pub fn clear(&mut self) {
        self.logs.clear();
    }

    pub fn view<'a, Message: Clone + 'a>(
        &self,
        on_resize_start: Message,
        on_input: impl Fn(String) -> Message + 'a,
        on_submit: Message,
        on_clear: Message,
    ) -> Element<'a, Message> {
        let (body_text, body_color) = if self.logs.is_empty() {
            ("Lua console logs will appear here.".to_string(), theme::MUTED)
        } else {
            let text = self.logs.iter().map(|line| line.as_str()).collect::<Vec<_>>().join("\n");
            (text, theme::TEXT_2)
        };

        // Grab handle drawn at the top edge, where dragging resizes the console.
        let grip = container(
            container(Space::new().width(36).height(3)).style(|_: &iced::Theme| container::Style {
                background: Some(iced::Background::Color(theme::BTN_BORDER)),
                border: iced::Border { radius: 2.0.into(), ..Default::default() },
                ..Default::default()
            }),
        )
        .center_x(Length::Fill)
        .height(Length::Fixed(RESIZE_EDGE_PX + 2.0))
        .align_y(iced::alignment::Vertical::Center);

        let header = row![
            text("LUA CONSOLE").size(11).font(theme::SANS_BOLD).color(theme::TEXT_2),
            Space::new().width(Length::Fill),
            button(text("Clear").size(11)).padding([2, 8]).style(btn(Btn::Ghost)).on_press(on_clear),
        ]
        .align_y(Alignment::Center)
        .padding([0, 10]);

        let prompt = row![
            text("›").size(14).font(theme::MONO_BOLD).color(theme::SELECT_RING),
            text_input("sim add blue 3", &self.input)
                .on_input(on_input)
                .on_submit(on_submit)
                .font(theme::MONO)
                .size(12)
                .padding([4, 8])
                .style(theme::input),
        ]
        .spacing(6)
        .padding([4, 10])
        .align_y(Alignment::Center);

        let panel: Element<'a, Message> = container(
            column![
                grip,
                header,
                scrollable(
                    container(
                        text(body_text)
                            .size(11.5)
                            .font(theme::MONO)
                            .color(body_color)
                            .width(Length::Fill)
                            .wrapping(text::Wrapping::Word),
                    )
                    .padding([2, 10]),
                )
                .id(self.scroll_id.clone())
                .height(Length::Fill),
                prompt,
            ]
            .spacing(2),
        )
        .width(Length::Fill)
        .height(Length::Fixed(self.height))
        .style(|_: &iced::Theme| container::Style {
            background: Some(iced::Background::Color(theme::CONSOLE)),
            text_color: Some(theme::TEXT),
            border: iced::Border { color: theme::DIVIDER, width: 1.0, radius: 0.0.into() },
            ..Default::default()
        })
        .into();

        let resize_handle: Element<'a, Message> = MouseArea::new(
            container(Space::new())
                .width(Length::Fill)
                .height(Length::Fixed(RESIZE_EDGE_PX)),
        )
        .on_press(on_resize_start)
        .interaction(mouse::Interaction::ResizingVertically)
        .into();

        iced::widget::stack![panel, resize_handle].into()
    }
}

fn max_lines_for_min_thumb(panel_height_px: f32) -> usize {
    // Clamp the content height so the scrollbar thumb does not shrink below a minimum size.
    let max_content_px = (panel_height_px * panel_height_px / MIN_SCROLLBAR_THUMB_PX)
        .max(panel_height_px);
    let max_lines = (max_content_px / APPROX_LINE_HEIGHT_PX).floor() as usize;
    max_lines.max(20)
}
