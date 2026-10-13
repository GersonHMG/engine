// gui/bottom_panel.rs — Robot selector + toggles, simulator summary and PPS meter

use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path, Stroke};
use iced::widget::{button, checkbox, container, row, text, tooltip, Space};
use iced::{mouse, Alignment, Element, Length, Point, Rectangle, Renderer, Theme};

use super::theme::{self, bar, btn, divider, group, tooltip_box, Btn};
use crate::gui::panels::control::Team;

const PPS_HISTORY: usize = 60;
/// Highest robot id the selector offers.
const MAX_ROBOT_ID: u32 = 12;

#[derive(Debug, Clone)]
pub enum BottomPanelMessage {
    SetTrace(bool),
    SetVectors(bool),
    /// Shows or hides the Lua drawings on the field.
    SetDraws(bool),
    SetManualControl(bool),
    TeamSelected(Team),
    IncrementRobotId,
    DecrementRobotId,
}

pub struct BottomPanel {
    pub trace_on: bool,
    pub vectors_on: bool,
    pub manual_control_on: bool,
    pub control_robot_id: String,
    pub control_team: Team,
    pub pps: u32,
    pub pps_history: Vec<u32>,
}

/// Simulator summary shown while the simulator is on: speed label and
/// robots per team (blue, yellow).
pub struct SimSummary {
    pub speed: String,
    pub robots: [usize; 2],
}

impl BottomPanel {
    pub fn new() -> Self {
        Self {
            trace_on: false,
            vectors_on: false,
            manual_control_on: false,
            control_robot_id: "0".to_string(),
            control_team: Team::Blue,
            pps: 0,
            pps_history: vec![0; PPS_HISTORY],
        }
    }

    pub fn push_pps(&mut self, pps: u32) {
        self.pps = pps;
        self.pps_history.remove(0);
        self.pps_history.push(pps);
    }

    /// `show_draws`: whether the Lua drawings are shown.
    pub fn view(&self, show_draws: bool, sim: Option<SimSummary>) -> Element<'_, BottomPanelMessage> {
        let enabled = self.manual_control_on;
        let id: u32 = self.control_robot_id.parse().unwrap_or(0);

        // Team swatch: filled in the team color, ringed while selected.
        let swatch = |team: Team| {
            let selected = self.control_team == team;
            let color = theme::team_color(team.to_id());
            button(Space::new().width(16).height(16))
                .padding(0)
                .style(move |th: &Theme, status| {
                    let mut style = btn(Btn::Neutral)(th, status);
                    let alpha = match (enabled, selected) {
                        (true, true) => 1.0,
                        (true, false) => 0.35,
                        (false, true) => 0.4,
                        (false, false) => 0.15,
                    };
                    style.background = Some(iced::Color { a: alpha, ..color }.into());
                    style.border.color = if selected && enabled { theme::TEXT } else { theme::BTN_BORDER };
                    style.border.width = if selected && enabled { 1.5 } else { 1.0 };
                    style
                })
                .on_press_maybe(enabled.then_some(BottomPanelMessage::TeamSelected(team)))
        };

        let step = |label: &'static str, message: Option<BottomPanelMessage>| {
            button(text(label).size(13).center())
                .width(22)
                .height(24)
                .padding(0)
                .style(btn(Btn::Ghost))
                .on_press_maybe(message)
        };
        let id_color = if enabled { theme::TEXT } else { theme::MUTED };
        let id_stepper = container(
            row![
                step("−", (enabled && id > 0).then_some(BottomPanelMessage::DecrementRobotId)),
                text(id).size(12).font(theme::MONO_BOLD).color(id_color).width(24).center(),
                step("+", (enabled && id < MAX_ROBOT_ID).then_some(BottomPanelMessage::IncrementRobotId)),
            ]
            .align_y(Alignment::Center),
        )
        .height(26)
        .center_y(26)
        .style(group);

        let selector = row![
            text("Robot").size(11).color(theme::MUTED),
            swatch(Team::Blue),
            swatch(Team::Yellow),
            id_stepper,
        ]
        .spacing(8)
        .align_y(Alignment::Center);
        let selector: Element<'_, BottomPanelMessage> = if enabled {
            selector.into()
        } else {
            tooltip(
                selector,
                container(text("Turn on Manual Control to pick a robot").size(12))
                    .padding([4, 8])
                    .style(tooltip_box),
                tooltip::Position::Top,
            )
            .into()
        };

        let check = |label: &'static str, on: bool, message: fn(bool) -> BottomPanelMessage| {
            checkbox(on).label(label).on_toggle(message).style(theme::check).size(14).text_size(12)
        };

        let mut content = row![
            selector,
            divider(),
            check("Manual Control", self.manual_control_on, BottomPanelMessage::SetManualControl),
            check("Trace", self.trace_on, BottomPanelMessage::SetTrace),
            check("Vectors", self.vectors_on, BottomPanelMessage::SetVectors),
            check("Lua draws", show_draws, BottomPanelMessage::SetDraws),
            Space::new().width(Length::Fill),
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        if let Some(sim) = sim {
            content = content
                .push(
                    text(format!("sim {} · {}B {}Y", sim.speed, sim.robots[0], sim.robots[1]))
                        .size(11)
                        .font(theme::MONO)
                        .color(theme::MUTED),
                )
                .push(divider());
        }

        content = content
            .push(Canvas::new(PpsSparkline { data: &self.pps_history }).width(60).height(16))
            .push(
                row![
                    text(self.pps).size(12).font(theme::MONO_BOLD),
                    text("PPS").size(11).color(theme::MUTED),
                ]
                .spacing(4)
                .align_y(Alignment::Center),
            );

        container(content)
            .width(Length::Fill)
            .height(34)
            .padding([0, 10])
            .center_y(34)
            .style(bar(theme::BAR))
            .into()
    }
}

/// Sparkline of the last PPS samples.
struct PpsSparkline<'a> {
    data: &'a [u32],
}

impl<'a, M> canvas::Program<M> for PpsSparkline<'a> {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let (w, h) = (bounds.width, bounds.height);
        let n = self.data.len();

        if n >= 2 {
            let max_pps = self.data.iter().copied().max().unwrap_or(1).max(1) as f32;
            let path = Path::new(|b| {
                for (i, v) in self.data.iter().enumerate() {
                    let point = Point::new(i as f32 / (n - 1) as f32 * w, h - 1.0 - *v as f32 / max_pps * (h - 2.0));
                    if i == 0 {
                        b.move_to(point);
                    } else {
                        b.line_to(point);
                    }
                }
            });
            frame.stroke(&path, Stroke::default().with_color(theme::SELECT_RING).with_width(1.2));
        }

        vec![frame.into_geometry()]
    }
}
