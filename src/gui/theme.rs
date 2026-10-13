// gui/theme.rs — Palette, fonts, icons and widget styles of the GUI.
//
// One meaning per color: GREEN = running/healthy, RED = recording, replay or
// failure, ORANGE = paused, ACCENT = "this toggle is on". Team colors are
// reserved for robots.

use iced::widget::{button, checkbox, container, svg, text_input, Space};
use iced::{color, font, Background, Border, Color, Element, Font, Length, Shadow, Theme};
use std::sync::LazyLock;

pub const SANS: Font = Font::with_name("IBM Plex Sans");
pub const SANS_BOLD: Font = Font { weight: font::Weight::Semibold, ..SANS };
pub const MONO: Font = Font::with_name("IBM Plex Mono");
pub const MONO_BOLD: Font = Font { weight: font::Weight::Semibold, ..MONO };

/// Font files embedded in the binary, registered at startup.
pub const FONTS: [&[u8]; 4] = [
    include_bytes!("../../fonts/IBMPlexSans-Regular.ttf"),
    include_bytes!("../../fonts/IBMPlexSans-SemiBold.ttf"),
    include_bytes!("../../fonts/IBMPlexMono-Regular.ttf"),
    include_bytes!("../../fonts/IBMPlexMono-SemiBold.ttf"),
];

pub const BG: Color = color!(0x141619);
pub const BAR: Color = color!(0x191c1f); // toolbar + bottom bar
pub const SIDEBAR: Color = color!(0x0f1113);
pub const PANEL: Color = color!(0x17191c);
pub const CONSOLE: Color = color!(0x0e1012);
pub const FIELD_BG: Color = color!(0x1a1d20);
pub const FIELD_FLOOR: Color = color!(0x1f2326);
pub const DIVIDER: Color = color!(0x262a2f);
pub const BTN: Color = color!(0x23272c);
pub const BTN_HOVER: Color = color!(0x2b3036);
pub const BTN_BORDER: Color = color!(0x353a41);
pub const GROUP: Color = color!(0x1d2024);
pub const INPUT: Color = color!(0x101214);
pub const INPUT_BORDER: Color = color!(0x33383e);
pub const TEXT: Color = color!(0xe6e8eb);
pub const TEXT_2: Color = color!(0xc9cdd2);
pub const MUTED: Color = color!(0x8a9098);

pub const ACCENT: Color = color!(0xa78bfa);
pub const ACCENT_DIM: Color = color!(0x3d3b55);
pub const ACCENT_HOVER: Color = color!(0x4b456a);
pub const ACCENT_FG: Color = color!(0xcfbffc);

pub const GREEN: Color = color!(0x4ade80);
pub const GREEN_DIM: Color = color!(0x143222);
pub const ORANGE: Color = color!(0xffb454);
pub const ORANGE_DIM: Color = color!(0x3a2a12);
pub const RED: Color = color!(0xff5c5c);
pub const RED_FG: Color = color!(0xff7b7b);
pub const RED_DIM: Color = color!(0x3d1719);
pub const REC_FILL: Color = color!(0xc9353b);
pub const REPLAY_BAR: Color = color!(0x211416);
pub const ON_RED: Color = color!(0x1a0b0c);

pub const SSL_BLUE: Color = color!(0x2f6bff);
pub const SSL_YELLOW: Color = color!(0xffd400);
pub const BALL: Color = color!(0xff8a1f);
pub const SELECT_RING: Color = color!(0x3ddc84);
pub const HEADING_TICK: Color = color!(0xff3b3b);
pub const VECTOR: Color = color!(0xf472b6);
pub const FIELD_LINE: Color = color!(0xe8eaed);

/// Color of a team's robots (0 = blue, 1 = yellow).
pub fn team_color(team: i32) -> Color {
    if team == 0 { SSL_BLUE } else { SSL_YELLOW }
}

/// The app theme: iced's dark theme built on this palette, so pick lists,
/// sliders, scrollbars and the panel windows follow it.
pub fn app_theme() -> Theme {
    static THEME: LazyLock<Theme> = LazyLock::new(|| {
        Theme::custom(
            "Sysmic",
            iced::theme::Palette {
                background: BG,
                text: TEXT,
                primary: ACCENT,
                success: GREEN,
                warning: ORANGE,
                danger: RED,
            },
        )
    });
    THEME.clone()
}

// --- Icons ---

#[derive(Debug, Clone, Copy)]
pub enum Icon {
    Chart,
    Eye,
    Filter,
    Folder,
    Gamepad,
    Play,
    Radio,
    Record,
    Reload,
    Replay,
    Stop,
    Terminal,
}

/// An icon, `size` px square. Stroked icons take the text color of their
/// button style when `tint` is given; play/stop keep their own colors.
pub fn icon<'a>(icon: Icon, size: f32, tint: Option<Color>) -> svg::Svg<'a> {
    static HANDLES: LazyLock<Vec<svg::Handle>> = LazyLock::new(|| {
        [
            include_bytes!("../../icons/ui/chart.svg").as_slice(),
            include_bytes!("../../icons/ui/eye.svg"),
            include_bytes!("../../icons/ui/filter.svg"),
            include_bytes!("../../icons/ui/folder.svg"),
            include_bytes!("../../icons/ui/gamepad.svg"),
            include_bytes!("../../icons/ui/play.svg"),
            include_bytes!("../../icons/ui/radio.svg"),
            include_bytes!("../../icons/ui/record.svg"),
            include_bytes!("../../icons/ui/reload.svg"),
            include_bytes!("../../icons/ui/replay.svg"),
            include_bytes!("../../icons/ui/stop.svg"),
            include_bytes!("../../icons/ui/terminal.svg"),
        ]
        .into_iter()
        .map(svg::Handle::from_memory)
        .collect()
    });
    svg(HANDLES[icon as usize].clone())
        .width(size)
        .height(size)
        .style(move |_, _| svg::Style { color: tint })
}

// --- Buttons ---

#[derive(Debug, Clone, Copy)]
pub enum Btn {
    Neutral,
    /// SIM, DRAWS: accent while on.
    Toggle(bool),
    /// REC: red while recording.
    Rec(bool),
    /// Sidebar icon: accent while its panel is open.
    Side(bool),
    /// Sidebar replay: red while in replay mode.
    Replay(bool),
    /// Steppers, menu items, close buttons.
    Ghost,
    /// Primary "go" action (replay play).
    Play,
}

fn border(c: Color) -> Border {
    Border { color: c, width: 1.0, radius: 4.0.into() }
}

pub fn btn(kind: Btn) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        let (bg, bd, fg, hbg) = match kind {
            Btn::Neutral | Btn::Toggle(false) | Btn::Rec(false) => (BTN, BTN_BORDER, TEXT_2, BTN_HOVER),
            Btn::Toggle(true) | Btn::Side(true) => (ACCENT_DIM, ACCENT, ACCENT_FG, ACCENT_HOVER),
            Btn::Rec(true) | Btn::Replay(true) => (REC_FILL, RED, Color::WHITE, color!(0xd63f45)),
            Btn::Side(false) | Btn::Replay(false) | Btn::Ghost => {
                (Color::TRANSPARENT, Color::TRANSPARENT, color!(0xb4bac1), BTN_HOVER)
            }
            Btn::Play => (GREEN_DIM, GREEN, GREEN, color!(0x1b4430)),
        };
        let mut style = button::Style {
            background: Some(Background::Color(if hovered { hbg } else { bg })),
            text_color: fg,
            border: border(bd),
            shadow: Shadow::default(),
            snap: true,
        };
        if let button::Status::Disabled = status {
            style.background = Some(Background::Color(bg));
            style.text_color = Color { a: 0.4, ..fg };
            style.border.color = Color { a: 0.4, ..bd };
        }
        style
    }
}

/// Text color a button of this kind gives its content, to tint its icon.
pub fn btn_fg(kind: Btn, enabled: bool) -> Color {
    let fg = btn(kind)(&app_theme(), button::Status::Active).text_color;
    if enabled { fg } else { Color { a: 0.4, ..fg } }
}

// --- Containers ---

pub fn bar(bg: Color) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(bg)),
        text_color: Some(TEXT),
        ..Default::default()
    }
}

/// Status pill: Running / Paused / Failed, REPLAY.
pub fn pill(bg: Color) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(bg)),
        border: Border { radius: 10.0.into(), ..Default::default() },
        ..Default::default()
    }
}

/// Grouped control: team stepper, robot id stepper.
pub fn group(_: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(GROUP)),
        border: border(BTN_BORDER),
        ..Default::default()
    }
}

pub fn tooltip_box(_: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(TEXT)),
        text_color: Some(color!(0x0c0d0f)),
        border: Border { radius: 4.0.into(), ..Default::default() },
        ..Default::default()
    }
}

/// Floating box over the field: coordinates, context menu, replay controls.
pub fn overlay_box(_: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(color!(0x1f2226))),
        border: border(color!(0x3a3f46)),
        text_color: Some(TEXT),
        ..Default::default()
    }
}

/// Red frame around the field in replay mode.
pub fn replay_frame(_: &Theme) -> container::Style {
    container::Style {
        border: Border { color: RED, width: 2.0, radius: 0.0.into() },
        ..Default::default()
    }
}

// --- Inputs ---

pub fn input(_: &Theme, status: text_input::Status) -> text_input::Style {
    let focused = matches!(status, text_input::Status::Focused { .. });
    text_input::Style {
        background: Background::Color(INPUT),
        border: border(if focused { color!(0x6b7280) } else { INPUT_BORDER }),
        icon: MUTED,
        placeholder: MUTED,
        value: TEXT,
        selection: ACCENT_DIM,
    }
}

pub fn check(_: &Theme, status: checkbox::Status) -> checkbox::Style {
    let (on, disabled) = match status {
        checkbox::Status::Active { is_checked } | checkbox::Status::Hovered { is_checked } => (is_checked, false),
        checkbox::Status::Disabled { is_checked } => (is_checked, true),
    };
    let alpha = if disabled { 0.4 } else { 1.0 };
    checkbox::Style {
        background: Background::Color(Color { a: alpha, ..if on { ACCENT } else { INPUT } }),
        icon_color: color!(0x111111),
        border: Border {
            color: Color { a: alpha, ..if on { ACCENT } else { color!(0x5a6068) } },
            width: 1.0,
            radius: 3.0.into(),
        },
        text_color: Some(Color { a: alpha, ..if on { TEXT } else { TEXT_2 } }),
    }
}

// --- Small pieces ---

/// Thin vertical divider between toolbar groups.
pub fn divider<'a, M: 'a>() -> Element<'a, M> {
    container(container(Space::new().width(1).height(18)).style(bar(DIVIDER)))
        .padding([0, 4])
        .into()
}

/// Filled circle of diameter `d`.
pub fn dot<'a, M: 'a>(c: Color, d: f32) -> Element<'a, M> {
    container(Space::new().width(d).height(d))
        .style(move |_| container::Style {
            background: Some(c.into()),
            border: Border { radius: (d / 2.0).into(), ..Default::default() },
            ..Default::default()
        })
        .width(Length::Shrink)
        .into()
}
