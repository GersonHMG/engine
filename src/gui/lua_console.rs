// gui/lua_console.rs — Lua console log panel UI

use iced::widget::{column, container, scrollable, text, Id, MouseArea, Space};
use iced::{Element, Length, mouse};
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
        }
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

    pub fn view<'a, Message: Clone + 'a>(
        &self,
        on_resize_start: Message,
    ) -> Element<'a, Message> {
        let (body_text, body_color) = if self.logs.is_empty() {
            (
                "Lua console logs will appear here.".to_string(),
                iced::Color::from_rgb(0.7, 0.7, 0.7),
            )
        } else {
            let text = self
                .logs
                .iter()
                .map(|line| line.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            (text, iced::Color::from_rgb(0.85, 0.85, 0.85))
        };

        let panel: Element<'a, Message> = container(
            column![
                text("Lua Console")
                    .size(12)
                    .color(iced::Color::from_rgb(0.8, 0.8, 0.8)),
                scrollable(
                    container(
                        text(body_text)
                            .size(11)
                            .color(body_color)
                            .width(Length::Fill)
                            .wrapping(text::Wrapping::Word),
                    )
                    .padding(6),
                )
                .id(self.scroll_id.clone())
                .height(Length::Fill),
            ]
            .spacing(4)
            .padding(6),
        )
        .width(Length::Fill)
        .height(Length::Fixed(self.height))
        .style(|theme: &iced::Theme| {
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
