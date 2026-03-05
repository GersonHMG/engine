// gui/field_canvas.rs — Canvas widget for the RoboCup SSL field

use iced::mouse;
use iced::widget::canvas::{self, Cache, Canvas, Frame, Geometry, Path, Stroke, Text};
use iced::{Color, Element, Length, Point, Rectangle, Size, Theme, Vector};


/// Data needed to render the field
#[derive(Debug, Clone, Default)]
pub struct FieldData {
    pub robots_blue: Vec<RobotData>,
    pub robots_yellow: Vec<RobotData>,
    pub ball: (f64, f64),
    pub path_points: Vec<(f64, f64)>,
    pub robot_trace: Vec<(f64, f64)>,
    pub lua_draw_commands: Vec<LuaDrawCommand>,
    pub vision_connected: bool,
    pub vis_velocities: bool,
    pub path_draw_mode: bool,
}

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

#[derive(Debug, Clone)]
pub enum LuaDrawCommand {
    Point { x: f64, y: f64 },
    HighlightRobot { id: i32, team: i32 },
    Line { points: Vec<(f64, f64)> },
}

#[derive(Debug, Clone)]
pub enum FieldMessage {
    CanvasClicked(f64, f64), // field coords
}

pub struct FieldCanvas {
    cache: Cache,
    pan: Vector,
    scale: f32,
    is_dragging: bool,
    last_mouse: Option<Point>,
    mouse_field_pos: Option<(f64, f64)>,
}

const FIELD_LENGTH: f32 = 9000.0;
const FIELD_WIDTH: f32 = 6000.0;
const MIN_SCALE: f32 = 0.01;
const MAX_SCALE: f32 = 0.5;

impl FieldCanvas {
    pub fn new() -> Self {
        Self {
            cache: Cache::new(),
            pan: Vector::new(0.0, 0.0),
            scale: 0.08,
            is_dragging: false,
            last_mouse: None,
            mouse_field_pos: None,
        }
    }

    pub fn request_redraw(&mut self) {
        self.cache.clear();
    }

    pub fn mouse_field_pos(&self) -> Option<(f64, f64)> {
        self.mouse_field_pos
    }

    pub fn view<'a, M: 'a>(&'a self, data: &'a FieldData) -> Element<'a, M> {
        Canvas::new(FieldProgram {
            data,
            pan: self.pan,
            scale: self.scale,
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    pub fn handle_scroll(&mut self, delta: f32) {
        let factor = 1.1f32;
        if delta > 0.0 {
            self.scale = (self.scale * factor).min(MAX_SCALE);
        } else {
            self.scale = (self.scale / factor).max(MIN_SCALE);
        }
        self.cache.clear();
    }

    pub fn handle_drag_start(&mut self, position: Point) {
        self.is_dragging = true;
        self.last_mouse = Some(position);
    }

    pub fn handle_drag_move(&mut self, position: Point) {
        if self.is_dragging {
            if let Some(last) = self.last_mouse {
                self.pan = self.pan + Vector::new(position.x - last.x, position.y - last.y);
                self.cache.clear();
            }
            self.last_mouse = Some(position);
        }
    }

    pub fn handle_drag_end(&mut self) -> bool {
        let was_dragging = self.is_dragging;
        let did_drag = if let Some(_) = self.last_mouse {
            was_dragging
        } else {
            false
        };
        self.is_dragging = false;
        self.last_mouse = None;
        did_drag
    }

    pub fn screen_to_field(&self, bounds: Rectangle, position: Point) -> (f64, f64) {
        let cx = bounds.width / 2.0 + self.pan.x;
        let cy = bounds.height / 2.0 + self.pan.y;
        let x = ((position.x - cx) / self.scale / 1000.0) as f64;
        let y = -((position.y - cy) / self.scale / 1000.0) as f64;
        (x, y)
    }

    pub fn update_mouse_pos(&mut self, bounds: Rectangle, position: Point) {
        self.mouse_field_pos = Some(self.screen_to_field(bounds, position));
    }
}

struct FieldProgram<'a> {
    data: &'a FieldData,
    pan: Vector,
    scale: f32,
}

impl<'a> FieldProgram<'a> {
    fn field_to_screen(&self, bounds: Rectangle, x: f64, y: f64) -> Point {
        let cx = bounds.width / 2.0 + self.pan.x;
        let cy = bounds.height / 2.0 + self.pan.y;
        Point::new(
            cx + (x as f32) * 1000.0 * self.scale,
            cy - (y as f32) * 1000.0 * self.scale,
        )
    }
}

impl<'a, M> canvas::Program<M> for FieldProgram<'a> {
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

        // Background
        frame.fill_rectangle(
            Point::ORIGIN,
            bounds.size(),
            Color::from_rgb(0.663, 0.663, 0.663),
        );

        let cx = bounds.width / 2.0 + self.pan.x;
        let cy = bounds.height / 2.0 + self.pan.y;
        let s = self.scale;

        // Field outline
        let field_rect = Path::rectangle(
            Point::new(cx - (FIELD_LENGTH / 2.0) * s, cy - (FIELD_WIDTH / 2.0) * s),
            Size::new(FIELD_LENGTH * s, FIELD_WIDTH * s),
        );
        frame.stroke(
            &field_rect,
            Stroke::default().with_color(Color::WHITE).with_width(2.0),
        );

        // Center circle
        let center_circle = Path::circle(Point::new(cx, cy), 500.0 * s);
        frame.stroke(
            &center_circle,
            Stroke::default().with_color(Color::WHITE).with_width(2.0),
        );

        // Center line
        let center_line = Path::line(
            Point::new(cx, cy - (FIELD_WIDTH / 2.0) * s),
            Point::new(cx, cy + (FIELD_WIDTH / 2.0) * s),
        );
        frame.stroke(
            &center_line,
            Stroke::default().with_color(Color::WHITE).with_width(2.0),
        );

        // Draw path waypoints
        if !self.data.path_points.is_empty() {
            let path_pts: Vec<Point> = self
                .data
                .path_points
                .iter()
                .map(|(x, y)| self.field_to_screen(bounds, *x, *y))
                .collect();

            // Lines
            if path_pts.len() >= 2 {
                let path = Path::new(|b| {
                    b.move_to(path_pts[0]);
                    for pt in &path_pts[1..] {
                        b.line_to(*pt);
                    }
                });
                frame.stroke(
                    &path,
                    Stroke::default()
                        .with_color(Color::from_rgb(1.0, 0.0, 1.0))
                        .with_width(2.0),
                );
            }

            // Dots
            for pt in &path_pts {
                let dot = Path::circle(*pt, 4.0);
                frame.fill(&dot, Color::from_rgb(1.0, 0.0, 1.0));
            }
        }

        // Draw robot trace
        if !self.data.robot_trace.is_empty() {
            let trace_pts: Vec<Point> = self
                .data
                .robot_trace
                .iter()
                .map(|(x, y)| self.field_to_screen(bounds, *x, *y))
                .collect();

            if trace_pts.len() >= 2 {
                let path = Path::new(|b| {
                    b.move_to(trace_pts[0]);
                    for pt in &trace_pts[1..] {
                        b.line_to(*pt);
                    }
                });
                frame.stroke(
                    &path,
                    Stroke::default()
                        .with_color(Color::from_rgb(0.0, 1.0, 1.0))
                        .with_width(2.0),
                );
            }
        }

        // Draw robots
        for robot in &self.data.robots_blue {
            self.draw_robot(&mut frame, bounds, robot, Color::from_rgb(0.1, 0.1, 0.9));
        }
        for robot in &self.data.robots_yellow {
            self.draw_robot(&mut frame, bounds, robot, Color::from_rgb(0.9, 0.9, 0.0));
        }

        // Draw ball
        {
            let pos = self.field_to_screen(bounds, self.data.ball.0, self.data.ball.1);
            let ball_path = Path::circle(pos, 25.0 * s);
            frame.fill(&ball_path, Color::from_rgb(1.0, 0.647, 0.0));
            frame.stroke(
                &ball_path,
                Stroke::default().with_color(Color::BLACK).with_width(0.5),
            );
        }

        // Draw Lua commands
        for cmd in &self.data.lua_draw_commands {
            match cmd {
                LuaDrawCommand::Point { x, y } => {
                    let pos = self.field_to_screen(bounds, *x, *y);
                    let r = (40.0 * s).max(4.0);
                    let path = Path::circle(pos, r);
                    frame.fill(&path, Color::from_rgb(0.0, 1.0, 0.0));
                }
                LuaDrawCommand::HighlightRobot { id, team } => {
                    let robots = if *team == 0 {
                        &self.data.robots_blue
                    } else {
                        &self.data.robots_yellow
                    };
                    if let Some(robot) = robots.iter().find(|r| r.id == *id as u32) {
                        let pos = self.field_to_screen(bounds, robot.x, robot.y);
                        let circle = Path::circle(pos, 120.0 * s);
                        frame.stroke(
                            &circle,
                            Stroke::default()
                                .with_color(Color::from_rgb(0.0, 1.0, 0.0))
                                .with_width(3.0),
                        );
                    }
                }
                LuaDrawCommand::Line { points } => {
                    if points.len() >= 2 {
                        let screen_pts: Vec<Point> = points
                            .iter()
                            .map(|(x, y)| self.field_to_screen(bounds, *x, *y))
                            .collect();
                        let path = Path::new(|b| {
                            b.move_to(screen_pts[0]);
                            for pt in &screen_pts[1..] {
                                b.line_to(*pt);
                            }
                        });
                        frame.stroke(
                            &path,
                            Stroke::default()
                                .with_color(Color::from_rgb(0.0, 1.0, 0.0))
                                .with_width(2.0),
                        );
                        for pt in &screen_pts {
                            let dot = Path::circle(*pt, (30.0 * s).max(3.0));
                            frame.fill(&dot, Color::from_rgb(0.0, 1.0, 0.0));
                        }
                    }
                }
            }
        }

        // Vision disconnected overlay
        if !self.data.vision_connected {
            frame.fill_rectangle(
                Point::ORIGIN,
                bounds.size(),
                Color::from_rgba(0.5, 0.5, 0.5, 0.5),
            );
            frame.fill_text(Text {
                content: "No Vision Connected".to_string(),
                position: Point::new(bounds.width / 2.0, bounds.height / 2.0),
                color: Color::WHITE,
                size: iced::Pixels(30.0),
                align_x: iced::alignment::Horizontal::Center.into(),
                align_y: iced::alignment::Vertical::Center.into(),
                ..Text::default()
            });
        }

        vec![frame.into_geometry()]
    }
}

impl<'a> FieldProgram<'a> {
    fn draw_robot(
        &self,
        frame: &mut Frame,
        bounds: Rectangle,
        robot: &RobotData,
        team_color: Color,
    ) {
        let pos = self.field_to_screen(bounds, robot.x, robot.y);
        let s = self.scale;
        let radius = 90.0 * s;

        // Robot body
        let body = Path::circle(pos, radius);
        frame.fill(&body, team_color);

        // Heading line
        let heading_end = Point::new(
            pos.x + radius * (robot.theta as f32).cos(),
            pos.y - radius * (robot.theta as f32).sin(),
        );
        let heading = Path::line(pos, heading_end);
        frame.stroke(
            &heading,
            Stroke::default().with_color(Color::BLACK).with_width(2.0),
        );

        // ID text
        frame.fill_text(Text {
            content: robot.id.to_string(),
            position: pos,
            color: Color::WHITE,
            size: iced::Pixels((12.0 * (s / 0.08)).max(10.0)),
            align_x: iced::alignment::Horizontal::Center.into(),
            align_y: iced::alignment::Vertical::Center.into(),
            ..Text::default()
        });

        // Velocity vectors
        if self.data.vis_velocities {
            let vel_scale = 1000.0 * s;

            // Actual velocity (red)
            if robot.vx.abs() > 0.05 || robot.vy.abs() > 0.05 {
                let end = Point::new(
                    pos.x + (robot.vx as f32) * vel_scale,
                    pos.y - (robot.vy as f32) * vel_scale,
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

            // Commanded velocity (green, local→global)
            if robot.cmd_vx.abs() > 0.05 || robot.cmd_vy.abs() > 0.05 {
                let theta = robot.theta as f32;
                let gvx = (robot.cmd_vx as f32) * theta.cos() - (robot.cmd_vy as f32) * theta.sin();
                let gvy = (robot.cmd_vx as f32) * theta.sin() + (robot.cmd_vy as f32) * theta.cos();
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
