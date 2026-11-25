// gui/panels/pipeline.rs — Vision → Kalman → Radio, as a compact list
//
// One small window for the whole data path: SSL vision packets come in, the
// Kalman filter cleans them up into the world state the AI reads, and the
// radio sends the AI's commands to the robots. Each stage is a row with its
// status; clicking it opens its settings. Between rows, a label shows
// whether data is flowing on to the next stage.

use iced::widget::{button, column, container, row, scrollable, text, text_input, toggler, Space};
use iced::{Alignment, Background, Border, Color, Element, Length};

use crate::gui::theme::{self, btn, icon, Btn, Icon};

/// What a stage is doing, shown at the right of its row.
pub struct Status {
    pub color: Color,
    pub label: String,
    /// The stage is passing data on; its icon takes the status color.
    pub live: bool,
}

impl Status {
    pub fn new(color: Color, label: impl Into<String>, live: bool) -> Self {
        Self { color, label: label.into(), live }
    }
}

/// One stage of the pipeline.
pub struct Stage<'a, M> {
    pub glyph: Icon,
    pub title: &'static str,
    pub role: &'static str,
    pub status: Status,
    /// Shown in small print under the settings (e.g. "used when SIM is off").
    pub note: Option<&'static str>,
    pub body: Element<'a, M>,
    pub action: (&'static str, M),
}

/// The connection between two stages.
pub struct Link {
    pub label: &'static str,
    pub live: bool,
    pub color: Color,
}

/// The list: stage, link, stage, link, stage. `open[i]` shows stage i's
/// settings; `on_toggle(i)` is sent when its row is clicked.
pub fn diagram<'a, M: Clone + 'a>(
    stages: [Stage<'a, M>; 3],
    links: [Link; 2],
    open: [bool; 3],
    on_toggle: impl Fn(usize) -> M,
) -> Element<'a, M> {
    let [a, b, c] = stages;
    let [ab, bc] = links;

    let list = column![
        stage_row(a, open[0], on_toggle(0)),
        link(ab),
        stage_row(b, open[1], on_toggle(1)),
        link(bc),
        stage_row(c, open[2], on_toggle(2)),
    ];

    container(scrollable(container(list).padding(10)))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(theme::BG)),
            text_color: Some(theme::TEXT),
            ..Default::default()
        })
        .into()
}

fn stage_row<'a, M: Clone + 'a>(stage: Stage<'a, M>, open: bool, toggle: M) -> Element<'a, M> {
    let Stage { glyph, title, role, status, note, body, action } = stage;
    let tint = if status.live { status.color } else { theme::TEXT_2 };

    let header = button(
        row![
            icon(glyph, 15.0, Some(tint)),
            text(title).size(13).font(theme::SANS_BOLD),
            text(role).size(10).font(theme::MONO).color(theme::MUTED),
            Space::new().width(Length::Fill),
            theme::dot(status.color, 6.0),
            text(status.label).size(11).font(theme::MONO).color(status.color),
            text(if open { "▾" } else { "▸" }).size(12).color(theme::MUTED),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .padding([7, 10])
    .style(btn(Btn::Ghost))
    .on_press(toggle);

    let mut content = column![header];
    if open {
        let mut settings = column![body].spacing(8);
        if let Some(note) = note {
            settings = settings.push(text(note).size(10).color(theme::MUTED));
        }
        let (label, message) = action;
        settings = settings.push(
            row![
                Space::new().width(Length::Fill),
                button(text(label).size(11)).padding([4, 12]).style(btn(Btn::Neutral)).on_press(message),
            ],
        );
        content = content.push(container(settings).padding(iced::Padding { top: 2.0, right: 12.0, bottom: 10.0, left: 12.0 }));
    }

    container(content)
        .width(Length::Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(theme::GROUP)),
            border: Border { color: theme::DIVIDER, width: 1.0, radius: 6.0.into() },
            text_color: Some(theme::TEXT),
            ..Default::default()
        })
        .into()
}

fn link<'a, M: 'a>(link: Link) -> Element<'a, M> {
    let color = if link.live { link.color } else { theme::MUTED };
    let label = if link.live { format!("↓ {}", link.label) } else { format!("↓ {} · idle", link.label) };
    container(text(label).size(10).font(theme::MONO).color(color))
        .padding(iced::Padding { top: 3.0, right: 0.0, bottom: 3.0, left: 18.0 })
        .into()
}

// --- Pieces for the stage bodies ---

/// A small caption over its control.
pub fn field<'a, M: 'a>(label: &'static str, control: impl Into<Element<'a, M>>) -> Element<'a, M> {
    column![text(label).size(10).font(theme::MONO).color(theme::MUTED), control.into()]
        .spacing(4)
        .into()
}

/// A text box in the stage's style.
pub fn input<'a, M: Clone + 'a>(placeholder: &str, value: &str, on_input: impl Fn(String) -> M + 'a) -> Element<'a, M> {
    text_input(placeholder, value)
        .on_input(on_input)
        .font(theme::MONO)
        .size(12)
        .padding([5, 8])
        .style(theme::input)
        .into()
}

/// A label with an on/off switch at the right.
pub fn switch<'a, M: 'a>(label: &'static str, on: bool, on_toggle: impl Fn(bool) -> M + 'a) -> Element<'a, M> {
    row![
        text(label).size(12).color(theme::TEXT_2),
        Space::new().width(Length::Fill),
        toggler(on).on_toggle(on_toggle).size(18),
    ]
    .align_y(Alignment::Center)
    .into()
}
