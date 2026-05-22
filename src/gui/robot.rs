use iced::widget::canvas::{Frame, Path, Stroke, Text};
use iced::{Color, Point};

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
    // Base colors for the filled body
    fn base_color(self) -> Color {
        match self {
            RobotTeam::Blue => Color::from_rgb(0.1, 0.1, 0.9),
            RobotTeam::Yellow => Color::from_rgb(229.0 / 255.0, 199.0 / 255.0, 65.0 / 255.0),
        }
    }

    // Brighter variant for the outline
    fn outline_color(self) -> Color {
        match self {
            // Brighter Blue
            RobotTeam::Blue => Color::from_rgb(0.4, 0.4, 1.0),
            // Brighter Yellow
            RobotTeam::Yellow => Color::from_rgb(255.0 / 255.0, 225.0 / 255.0, 110.0 / 255.0),
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
        let team_color = self.team.base_color();
        let outline_color = self.team.outline_color();

        // 1. Draw Green Highlight Circle (if highlighted)
        if self.highlighted {
            // Make the ring slightly larger than the robot body
            let highlight_radius = radius + (5.0 * scale).max(2.0);
            let highlight_ring = Path::circle(pos, highlight_radius);
            
            frame.stroke(
                &highlight_ring,
                Stroke::default()
                    .with_color(Color::from_rgb(0.0, 1.0, 0.0)) // Green
                    .with_width(1.0), // Adjust thickness as needed
            );
        }

        // 2. Robot body (flattened at the top)
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
        
        // Fill the body with the base color
        frame.fill(&body, team_color);
        
        // Draw the bright outline over the body
        frame.stroke(
            &body,
            Stroke::default()
                .with_color(outline_color)
                .with_width(2.0), // Adjust the outline thickness as needed
        );

        // Heading line (adjusted so 0 radians points UP instead of RIGHT)
        // Note: The + FRAC_PI_2 belongs inside the cos()/sin() functions, not outside!
        let visual_theta = self.data.theta as f32;
        let heading_end = Point::new(
            pos.x + radius * visual_theta.cos(),
            pos.y - radius * visual_theta.sin(),
        );
        
        let heading = Path::line(pos, heading_end);
        frame.stroke(
            &heading,
            Stroke::default()
                .with_color(Color::from_rgb(0.8, 0.0, 0.0))
                .with_width(2.0),
        );

        // ID text
        frame.fill_text(Text {
            content: self.data.id.to_string(),
            position: pos,
            color: Color::WHITE,
            size: iced::Pixels((12.0 * (scale / 0.08)).max(10.0)),
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
                        .with_color(Color::from_rgba(1.0, 0.0, 0.0, 0.5))
                        .with_width(3.0),
                );
                let dot = Path::circle(end, 3.0);
                frame.fill(&dot, Color::from_rgba(1.0, 0.0, 0.0, 0.5));
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
                        .with_color(Color::from_rgba(0.0, 1.0, 0.0, 0.5))
                        .with_width(3.0),
                );
                let dot = Path::circle(end, 3.0);
                frame.fill(&dot, Color::from_rgba(0.0, 1.0, 0.0, 0.5));
            }
        }
    }
}