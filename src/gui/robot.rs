use iced::widget::canvas::{Frame, Path, Stroke, Text};
use iced::{Color, Point, Vector};

use super::theme;

#[derive(Debug, Clone)]
pub struct RobotData {
    pub id: u32,
    pub x: f64,
    pub y: f64,
    pub theta: f64,
    pub vx: f64,
    pub vy: f64,
    pub cmd_vx: f64,
    pub cmd_vy: f64,
    pub cmd_angular: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RobotTeam {
    Blue,
    Yellow,
}

impl RobotTeam {
    fn color(self) -> Color {
        match self {
            RobotTeam::Blue => theme::SSL_BLUE,
            RobotTeam::Yellow => theme::SSL_YELLOW,
        }
    }

    /// ID text color, readable on the team color.
    fn id_color(self) -> Color {
        match self {
            RobotTeam::Blue => Color::WHITE,
            RobotTeam::Yellow => Color::BLACK,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RobotGui<'a> {
    pub data: &'a RobotData,
    pub team: RobotTeam,
    pub highlighted: bool,
}

impl<'a> RobotGui<'a> {
    pub fn new(data: &'a RobotData, team: RobotTeam, highlighted: bool) -> Self {
        Self {
            data,
            team,
            highlighted,
        }
    }

    pub fn draw(&self, frame: &mut Frame, pos: Point, scale: f32, show_velocities: bool) {
        let radius = 90.0 * scale;

        // Selection ring
        if self.highlighted {
            let ring = Path::circle(pos, radius + (45.0 * scale).max(3.0));
            frame.stroke(&ring, Stroke::default().with_color(theme::SELECT_RING).with_width(2.0));
        }

        // Robot body (flattened at the front)
        let body = Path::new(|builder| {
            let canvas_theta = -(self.data.theta as f32);
            let flat_angle = std::f32::consts::FRAC_PI_6;

            builder.arc(iced::widget::canvas::path::Arc {
                center: pos,
                radius,
                start_angle: iced::Radians(canvas_theta + flat_angle),
                end_angle: iced::Radians(canvas_theta + std::f32::consts::TAU - flat_angle),
            });

            builder.close();
        });
        frame.fill(&body, self.team.color());

        // Heading: short tick at the front edge
        let visual_theta = self.data.theta as f32;
        let dir = Vector::new(visual_theta.cos(), -visual_theta.sin());
        let heading = Path::line(pos + dir * (radius * 0.55), pos + dir * radius);
        frame.stroke(&heading, Stroke::default().with_color(theme::HEADING_TICK).with_width(2.4));

        // ID text
        frame.fill_text(Text {
            content: self.data.id.to_string(),
            position: pos,
            color: self.team.id_color(),
            size: iced::Pixels((11.0 * (scale / 0.08)).max(9.0)),
            font: theme::MONO_BOLD,
            align_x: iced::alignment::Horizontal::Center.into(),
            align_y: iced::alignment::Vertical::Center.into(),
            ..Text::default()
        });

        if show_velocities {
            let vel_scale = 1000.0 * scale;

            // Actual velocity (red)
            if self.data.vx.abs() > 0.05 || self.data.vy.abs() > 0.05 {
                let end = Point::new(
                    pos.x + (self.data.vx as f32) * vel_scale,
                    pos.y - (self.data.vy as f32) * vel_scale,
                );
                let line = Path::line(pos, end);
                frame.stroke(
                    &line,
                    Stroke::default()
                        .with_color(theme::VECTOR)
                        .with_width(1.6),
                );
                let dot = Path::circle(end, 2.0);
                frame.fill(&dot, theme::VECTOR);
            }

            // Commanded velocity (green, local to global)
            if self.data.cmd_vx.abs() > 0.05 || self.data.cmd_vy.abs() > 0.05 {
                let theta = visual_theta; 
                let gvx = (self.data.cmd_vx as f32) * theta.cos()
                    - (self.data.cmd_vy as f32) * theta.sin();
                let gvy = (self.data.cmd_vx as f32) * theta.sin()
                    + (self.data.cmd_vy as f32) * theta.cos();
                let end = Point::new(pos.x + gvx * vel_scale, pos.y - gvy * vel_scale);
                let line = Path::line(pos, end);
                frame.stroke(
                    &line,
                    Stroke::default()
                        .with_color(Color { a: 0.7, ..theme::GREEN })
                        .with_width(1.6),
                );
                let dot = Path::circle(end, 2.0);
                frame.fill(&dot, Color { a: 0.7, ..theme::GREEN });
            }
        }
    }
}