//! Canvas drawing of the field, the robots and the ball.

use iced::mouse;
use iced::widget::canvas::{self, Frame, Geometry, Path, Stroke};
use iced::{Color, Point, Rectangle, Renderer, Size, Theme};
use ssl_sim::{Robot, SimConfig, SimState, Team, Vec2};

const MARGIN_PX: f32 = 10.0;
/// Points on the robot's round side.
const ROBOT_ARC_POINTS: usize = 32;

const BACKGROUND: Color = Color::from_rgb(0.12, 0.12, 0.12);
const GRASS: Color = Color::from_rgb(0.13, 0.42, 0.18);
const LINES: Color = Color::WHITE;
const WALLS: Color = Color::from_rgb(0.55, 0.55, 0.55);
const BLUE_ROBOT: Color = Color::from_rgb(0.2, 0.4, 0.95);
const YELLOW_ROBOT: Color = Color::from_rgb(0.95, 0.85, 0.2);
const BALL: Color = Color::from_rgb(1.0, 0.55, 0.0);

pub struct FieldView<'a> {
    pub state: &'a SimState,
    pub config: &'a SimConfig,
}

impl<Message> canvas::Program<Message> for FieldView<'_> {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), BACKGROUND);

        let view = Viewport::fit(bounds.size(), self.config);
        self.draw_field(&mut frame, &view);
        for robot in self.state.robots() {
            self.draw_robot(&mut frame, &view, robot);
        }
        self.draw_ball(&mut frame, &view);

        vec![frame.into_geometry()]
    }
}

impl FieldView<'_> {
    fn draw_field(&self, frame: &mut Frame, view: &Viewport) {
        let field = &self.config.field;
        let (wall_x, wall_y) = (field.wall_x(), field.wall_y());
        let (half_length, half_width) = (field.length / 2.0, field.width / 2.0);

        let walls = view.rectangle(Vec2::new(-wall_x, wall_y), Vec2::new(wall_x, -wall_y));
        frame.fill(&walls, GRASS);
        frame.stroke(&walls, Stroke::default().with_color(WALLS).with_width(4.0));

        let lines = Stroke::default().with_color(LINES).with_width(1.5);
        let playing_area = view.rectangle(
            Vec2::new(-half_length, half_width),
            Vec2::new(half_length, -half_width),
        );
        frame.stroke(&playing_area, lines);
        frame.stroke(
            &Path::line(
                view.to_screen(Vec2::new(0.0, half_width)),
                view.to_screen(Vec2::new(0.0, -half_width)),
            ),
            lines,
        );
        frame.stroke(&Path::circle(view.to_screen(Vec2::ZERO), 0.5 * view.scale), lines);
    }

    /// The body outline: the round side as an arc, closed by the flat front.
    fn draw_robot(&self, frame: &mut Frame, view: &Viewport, robot: &Robot) {
        let config = &self.config.robot;
        let half_angle = (config.front_distance / config.radius).acos();

        let body = Path::new(|path| {
            for i in 0..=ROBOT_ARC_POINTS {
                let a = half_angle + (2.0 * std::f64::consts::PI - 2.0 * half_angle) * i as f64 / ROBOT_ARC_POINTS as f64;
                let local = Vec2::new(config.radius * a.cos(), config.radius * a.sin());
                let point = view.to_screen(robot.position + local.rotated(robot.theta));
                if i == 0 {
                    path.move_to(point);
                } else {
                    path.line_to(point);
                }
            }
            path.close();
        });
        let color = match robot.id.team {
            Team::Blue => BLUE_ROBOT,
            Team::Yellow => YELLOW_ROBOT,
        };
        frame.fill(&body, color);
        frame.stroke(&body, Stroke::default().with_color(LINES).with_width(1.0));

        let front = robot.position + robot.heading() * config.front_distance;
        frame.stroke(
            &Path::line(view.to_screen(robot.position), view.to_screen(front)),
            Stroke::default().with_color(LINES).with_width(2.0),
        );
    }

    fn draw_ball(&self, frame: &mut Frame, view: &Viewport) {
        let ball = &self.state.ball;
        let radius = (self.config.ball.radius as f32 * view.scale).max(3.0);
        frame.fill(&Path::circle(view.to_screen(ball.position), radius), BALL);
    }
}

/// Maps field meters to screen pixels, fitting the walls in the canvas.
struct Viewport {
    center: Point,
    /// Pixels per meter.
    scale: f32,
}

impl Viewport {
    fn fit(size: Size, config: &SimConfig) -> Self {
        let width_m = 2.0 * config.field.wall_x() as f32;
        let height_m = 2.0 * config.field.wall_y() as f32;
        let scale = ((size.width - 2.0 * MARGIN_PX) / width_m)
            .min((size.height - 2.0 * MARGIN_PX) / height_m)
            .max(1.0);
        Self {
            center: Point::new(size.width / 2.0, size.height / 2.0),
            scale,
        }
    }

    /// Field y points up, screen y points down.
    fn to_screen(&self, p: Vec2) -> Point {
        Point::new(
            self.center.x + p.x as f32 * self.scale,
            self.center.y - p.y as f32 * self.scale,
        )
    }

    fn rectangle(&self, top_left: Vec2, bottom_right: Vec2) -> Path {
        let a = self.to_screen(top_left);
        let b = self.to_screen(bottom_right);
        Path::rectangle(a, Size::new(b.x - a.x, b.y - a.y))
    }
}
