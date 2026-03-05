// gui/bottom_panel.rs — Velocity/position charts + toggle buttons + robot selector

use iced::widget::canvas::{self, Frame, Geometry, Path, Stroke, Text};
use iced::widget::{button, column, container, pick_list, row, text, text_input, Canvas};
use iced::{mouse, Color, Element, Length, Point, Rectangle, Size, Theme};

use crate::gui::panels::control::Team;

const CHART_HISTORY_SIZE: usize = 600;

#[derive(Debug, Clone)]
pub enum BottomPanelMessage {
    ToggleCapture,
    ToggleTrace,
    ToggleVectors,
    TeamSelected(Team),
    RobotIdChanged(String),
}

#[derive(Debug, Clone)]
pub struct ChartData {
    pub vx: Vec<f64>,
    pub vy: Vec<f64>,
    pub omega: Vec<f64>,
    pub x: Vec<f64>,
    pub y: Vec<f64>,
    pub theta: Vec<f64>,
}

impl Default for ChartData {
    fn default() -> Self {
        Self {
            vx: vec![0.0; CHART_HISTORY_SIZE],
            vy: vec![0.0; CHART_HISTORY_SIZE],
            omega: vec![0.0; CHART_HISTORY_SIZE],
            x: vec![0.0; CHART_HISTORY_SIZE],
            y: vec![0.0; CHART_HISTORY_SIZE],
            theta: vec![0.0; CHART_HISTORY_SIZE],
        }
    }
}

impl ChartData {
    pub fn push_vel(&mut self, vx: f64, vy: f64, omega: f64) {
        self.vx.remove(0);
        self.vx.push(vx);
        self.vy.remove(0);
        self.vy.push(vy);
        self.omega.remove(0);
        self.omega.push(omega);
    }

    pub fn push_pos(&mut self, x: f64, y: f64, theta: f64) {
        self.x.remove(0);
        self.x.push(x);
        self.y.remove(0);
        self.y.push(y);
        self.theta.remove(0);
        self.theta.push(theta);
    }
}

pub struct BottomPanel {
    pub capturing: bool,
    pub trace_on: bool,
    pub vectors_on: bool,
    pub control_robot_id: String,
    pub control_team: Team,
    pub chart_data: ChartData,
}

impl BottomPanel {
    pub fn new() -> Self {
        Self {
            capturing: false,
            trace_on: false,
            vectors_on: false,
            control_robot_id: "0".to_string(),
            control_team: Team::Blue,
            chart_data: ChartData::default(),
        }
    }

    pub fn view(&self) -> Element<BottomPanelMessage> {
        let controls = column![
            // Robot selector
            row![
                text("Team").size(10),
                pick_list(
                    &[Team::Blue, Team::Yellow][..],
                    Some(self.control_team),
                    BottomPanelMessage::TeamSelected,
                )
                .text_size(10)
                .width(Length::Fill),
            ]
            .spacing(4)
            .align_y(iced::Alignment::Center),
            row![
                text("ID").size(10),
                text_input("0", &self.control_robot_id)
                    .on_input(BottomPanelMessage::RobotIdChanged)
                    .size(10)
                    .width(Length::Fixed(40.0)),
            ]
            .spacing(4)
            .align_y(iced::Alignment::Center),
            // Toggle buttons
            button(
                text(if self.capturing {
                    "Capture ON"
                } else {
                    "Capture OFF"
                })
                .size(10)
                .center()
                .width(Length::Fill),
            )
            .on_press(BottomPanelMessage::ToggleCapture)
            .width(Length::Fill)
            .style(if self.capturing {
                button::primary
            } else {
                button::secondary
            }),
            button(
                text(if self.trace_on {
                    "Trace ON"
                } else {
                    "Trace OFF"
                })
                .size(10)
                .center()
                .width(Length::Fill),
            )
            .on_press(BottomPanelMessage::ToggleTrace)
            .width(Length::Fill)
            .style(if self.trace_on {
                button::primary
            } else {
                button::secondary
            }),
            button(
                text(if self.vectors_on {
                    "Vectors ON"
                } else {
                    "Vectors OFF"
                })
                .size(10)
                .center()
                .width(Length::Fill),
            )
            .on_press(BottomPanelMessage::ToggleVectors)
            .width(Length::Fill)
            .style(if self.vectors_on {
                button::primary
            } else {
                button::secondary
            }),
        ]
        .spacing(4)
        .width(Length::Fixed(100.0));

        // Always show charts — active = colored, inactive = gray
        let active = self.capturing;

        let chart_color = |c: Color| -> Color {
            if active { c } else { Color::from_rgb(0.35, 0.35, 0.4) }
        };
        let label_color = |c: Color| -> Color {
            if active { c } else { Color::from_rgb(0.4, 0.4, 0.45) }
        };

        let vx_color = Color::from_rgb(1.0, 0.27, 0.27);
        let vy_color = Color::from_rgb(0.27, 1.0, 0.27);
        let omega_color = Color::from_rgb(0.27, 0.53, 1.0);
        let x_color = Color::from_rgb(1.0, 0.53, 0.27);
        let y_color = Color::from_rgb(0.27, 0.87, 1.0);
        let theta_color = Color::from_rgb(0.87, 0.53, 1.0);

        let charts = row![
            column![
                text("Vx").size(9).color(label_color(vx_color)),
                Canvas::new(ChartProgram {
                    data: &self.chart_data.vx,
                    color: chart_color(vx_color),
                    active,
                })
                .width(Length::Fill)
                .height(Length::Fixed(36.0)),
                text("X").size(9).color(label_color(x_color)),
                Canvas::new(ChartProgram {
                    data: &self.chart_data.x,
                    color: chart_color(x_color),
                    active,
                })
                .width(Length::Fill)
                .height(Length::Fixed(36.0)),
            ]
            .spacing(2)
            .width(Length::Fill),
            column![
                text("Vy").size(9).color(label_color(vy_color)),
                Canvas::new(ChartProgram {
                    data: &self.chart_data.vy,
                    color: chart_color(vy_color),
                    active,
                })
                .width(Length::Fill)
                .height(Length::Fixed(36.0)),
                text("Y").size(9).color(label_color(y_color)),
                Canvas::new(ChartProgram {
                    data: &self.chart_data.y,
                    color: chart_color(y_color),
                    active,
                })
                .width(Length::Fill)
                .height(Length::Fixed(36.0)),
            ]
            .spacing(2)
            .width(Length::Fill),
            column![
                text("ω").size(9).color(label_color(omega_color)),
                Canvas::new(ChartProgram {
                    data: &self.chart_data.omega,
                    color: chart_color(omega_color),
                    active,
                })
                .width(Length::Fill)
                .height(Length::Fixed(36.0)),
                text("θ").size(9).color(label_color(theta_color)),
                Canvas::new(ChartProgram {
                    data: &self.chart_data.theta,
                    color: chart_color(theta_color),
                    active,
                })
                .width(Length::Fill)
                .height(Length::Fixed(36.0)),
            ]
            .spacing(2)
            .width(Length::Fill),
        ]
        .spacing(4)
        .width(Length::Fill);

        let content = row![controls, charts,]
            .spacing(8)
            .padding(8)
            .align_y(iced::Alignment::Center);

        container(content)
            .width(Length::Fill)
            .height(Length::Fixed(180.0))
            .style(panel_style)
            .into()
    }
}

struct ChartProgram<'a> {
    data: &'a [f64],
    color: Color,
    active: bool,
}

impl<'a> canvas::Program<BottomPanelMessage> for ChartProgram<'a> {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let w = bounds.width;
        let h = bounds.height;

        let bg = if self.active {
            Color::from_rgb(0.1, 0.1, 0.18)
        } else {
            Color::from_rgb(0.12, 0.12, 0.14)
        };
        frame.fill_rectangle(Point::ORIGIN, Size::new(w, h), bg);

        if self.data.is_empty() {
            return vec![frame.into_geometry()];
        }

        let mut min_val = f64::INFINITY;
        let mut max_val = f64::NEG_INFINITY;
        for v in self.data {
            if *v < min_val { min_val = *v; }
            if *v > max_val { max_val = *v; }
        }
        let range = max_val - min_val;
        let padding = if range < 0.01 { 1.0 } else { range * 0.15 };
        let y_min = min_val - padding;
        let y_max = max_val + padding;
        let y_range = y_max - y_min;

        if self.data.len() >= 2 && y_range > 0.0 {
            let path = Path::new(|b| {
                for (i, v) in self.data.iter().enumerate() {
                    let px = (i as f32 / (self.data.len() - 1) as f32) * w;
                    let py = h - ((*v - y_min) / y_range) as f32 * h;
                    if i == 0 {
                        b.move_to(Point::new(px, py));
                    } else {
                        b.line_to(Point::new(px, py));
                    }
                }
            });
            frame.stroke(
                &path,
                Stroke::default().with_color(self.color).with_width(1.5),
            );
        }

        if let Some(last) = self.data.last() {
            let val_color = if self.active {
                Color::from_rgb(0.67, 0.67, 0.67)
            } else {
                Color::from_rgb(0.35, 0.35, 0.38)
            };
            frame.fill_text(Text {
                content: format!("{:.2}", last),
                position: Point::new(w - 3.0, h - 3.0),
                color: val_color,
                size: iced::Pixels(9.0),
                align_x: iced::alignment::Horizontal::Right.into(),
                align_y: iced::alignment::Vertical::Bottom.into(),
                ..Text::default()
            });
        }

        vec![frame.into_geometry()]
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
