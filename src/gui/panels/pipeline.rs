// gui/panels/pipeline.rs — Vision → Kalman → Radio, drawn as a diagram
//
// One window for the whole data path: SSL vision packets come in, the Kalman
// filter cleans them up into the world state the AI reads, and the radio sends
// the AI's commands to the robots. Each stage is a card with its status and
// settings; animated arrows show where data is flowing right now.

use iced::widget::canvas::{self, Canvas, Frame, Geometry, LineDash, Path, Stroke};
use iced::widget::{button, column, container, row, text, text_input, toggler, Space};
use iced::{mouse, Alignment, Background, Border, Color, Element, Length, Point, Rectangle, Renderer, Shadow, Theme, Vector};
use std::sync::LazyLock;
use std::time::Instant;

use crate::gui::theme::{self, btn, icon, Btn, Icon};

/// What a stage is doing, shown as a pill on its card.
pub struct Status {
    pub color: Color,
    pub label: String,
    /// The stage is passing data on; its card gets a colored frame.
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

/// An arrow between two stages.
pub struct Link {
    pub label: &'static str,
    pub live: bool,
    pub color: Color,
}

pub const CARD_WIDTH: f32 = 232.0;

/// The diagram: stage, link, stage, link, stage.
pub fn diagram<'a, M: Clone + 'a>(stages: [Stage<'a, M>; 3], links: [Link; 2]) -> Element<'a, M> {
    let [a, b, c] = stages;
    let [ab, bc] = links;

    let header = row![
        column![
            text("Data pipeline").size(16).font(theme::SANS_BOLD),
            text("Packets in, filtered state to the AI, commands out to the robots")
                .size(11)
                .color(theme::MUTED),
        ]
        .spacing(2),
    ];

    let flow = row![card(a), link(ab), card(b), link(bc), card(c)]
        .align_y(Alignment::Center)
        .height(Length::Fill);

    container(column![header, flow].spacing(14))
        .padding([14, 16])
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(theme::BG)),
            text_color: Some(theme::TEXT),
            ..Default::default()
        })
        .into()
}

fn card<'a, M: Clone + 'a>(stage: Stage<'a, M>) -> Element<'a, M> {
    let Stage { glyph, title, role, status, note, body, action } = stage;
    let live = status.live;
    let accent = status.color;

    let badge = container(icon(glyph, 18.0, Some(if live { accent } else { theme::TEXT_2 })))
        .center(34)
        .style(move |_| container::Style {
            background: Some(Background::Color(Color { a: if live { 0.14 } else { 0.06 }, ..if live { accent } else { theme::TEXT } })),
            border: Border { color: Color { a: if live { 0.45 } else { 0.0 }, ..accent }, width: 1.0, radius: 8.0.into() },
            ..Default::default()
        });

    let pill = container(
        row![theme::dot(status.color, 7.0), text(status.label).size(11).font(theme::MONO).color(status.color)]
            .spacing(6)
            .align_y(Alignment::Center),
    )
    .padding([3, 9])
    .style(move |_| container::Style {
        background: Some(Background::Color(Color { a: 0.12, ..accent })),
        border: Border { radius: 10.0.into(), ..Default::default() },
        ..Default::default()
    });

    let head = row![
        badge,
        column![
            text(title).size(15).font(theme::SANS_BOLD),
            text(role).size(10).font(theme::MONO).color(theme::MUTED),
        ]
        .spacing(1),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    let rule = container(Space::new().width(Length::Fill).height(1)).style(theme::bar(theme::DIVIDER));

    let mut content = column![head, pill, rule, body].spacing(10);
    if let Some(note) = note {
        content = content.push(text(note).size(10).color(theme::MUTED));
    }
    let (label, message) = action;
    content = content.push(Space::new().height(Length::Fill)).push(
        button(text(label).size(12).center().width(Length::Fill))
            .padding([6, 8])
            .width(Length::Fill)
            .style(btn(Btn::Neutral))
            .on_press(message),
    );

    container(content)
        .padding(14)
        .width(CARD_WIDTH)
        .height(Length::Fill)
        .style(move |_| container::Style {
            background: Some(Background::Color(iced::color!(0x1b1e22))),
            text_color: Some(theme::TEXT),
            border: Border {
                color: if live { Color { a: 0.5, ..accent } } else { theme::BTN_BORDER },
                width: 1.0,
                radius: 12.0.into(),
            },
            shadow: Shadow {
                color: if live { Color { a: 0.18, ..accent } } else { Color { a: 0.35, ..Color::BLACK } },
                offset: Vector::new(0.0, 4.0),
                blur_radius: 18.0,
            },
            snap: true,
        })
        .into()
}

fn link<'a, M: 'a>(link: Link) -> Element<'a, M> {
    let color = if link.live { link.color } else { theme::MUTED };
    column![
        text(link.label).size(10).font(theme::MONO).color(color),
        Canvas::new(Flow { live: link.live, color: link.color }).width(Length::Fill).height(22),
    ]
    .spacing(2)
    .align_x(Alignment::Center)
    .width(Length::Fill)
    .into()
}

/// Arrow with dots running along it while data flows.
struct Flow {
    live: bool,
    color: Color,
}

impl<M> canvas::Program<M> for Flow {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        static START: LazyLock<Instant> = LazyLock::new(Instant::now);
        let mut frame = Frame::new(renderer, bounds.size());
        let (w, y) = (bounds.width, bounds.height / 2.0);
        let (x0, x1) = (6.0, w - 6.0);
        let head = 7.0;

        let line = Path::line(Point::new(x0, y), Point::new(x1 - head, y));
        let arrow = Path::new(|b| {
            b.move_to(Point::new(x1, y));
            b.line_to(Point::new(x1 - head, y - 4.5));
            b.line_to(Point::new(x1 - head, y + 4.5));
            b.close();
        });

        if self.live {
            frame.stroke(&line, Stroke::default().with_color(Color { a: 0.35, ..self.color }).with_width(2.0));
            frame.fill(&arrow, self.color);
            // Three dots, one every third of the way, moving left to right.
            let phase = (START.elapsed().as_secs_f32() * 0.8).fract();
            let span = x1 - head - x0;
            for k in 0..3 {
                let t = (phase + k as f32 / 3.0).fract();
                let fade = (t * 6.0).min((1.0 - t) * 6.0).min(1.0);
                let dot = Path::circle(Point::new(x0 + t * span, y), 2.6);
                frame.fill(&dot, Color { a: fade, ..self.color });
            }
        } else {
            let dashed = Stroke {
                line_dash: LineDash { segments: &[4.0, 4.0], offset: 0 },
                ..Stroke::default().with_color(Color { a: 0.5, ..theme::MUTED }).with_width(1.5)
            };
            frame.stroke(&line, dashed);
            frame.fill(&arrow, Color { a: 0.5, ..theme::MUTED });
        }

        vec![frame.into_geometry()]
    }
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
