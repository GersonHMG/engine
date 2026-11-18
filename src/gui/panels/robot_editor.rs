// gui/panels/robot_editor.rs — Edit the simulator's robot and save it as a profile
//
// One robot, three views of it:
// - a top-down drawing to scale (body, kicker, kick zone, ball, the four omni
//   wheels); drag a wheel around the body to change its angle;
// - its speed envelope: how fast it can drive in each direction, which the
//   wheel angles shape once the wheels have a top speed;
// - every parameter as a field, grouped, highlighted while it differs from
//   what the simulator runs.
// Apply sends the changes to the running simulator (`sim robot <name> <value>`);
// Save writes robots/<name>.toml (robot_profiles), Open reads one back.

use iced::alignment::{Horizontal, Vertical};
use iced::mouse;
use iced::widget::canvas::{self, path::Arc, Canvas, Frame, Geometry, LineDash, Path, Stroke, Text};
use iced::widget::{button, column, container, pick_list, row, text, text_input, Space};
use iced::{Alignment, Background, Border, Color, Element, Length, Point, Rectangle, Renderer, Size, Theme, Vector};
use ssl_sim::{RobotConfig, SimConfig};
use std::collections::BTreeMap;
use std::f64::consts::PI;

use crate::gui::theme::{self, btn, Btn};
use crate::robot_profiles;

/// One parameter: config name, label, unit shown, and whether the field is in
/// degrees (the config is in radians).
struct Param {
    name: &'static str,
    label: &'static str,
    unit: &'static str,
    degrees: bool,
}

const fn p(name: &'static str, label: &'static str, unit: &'static str) -> Param {
    Param { name, label, unit, degrees: false }
}

const fn deg(name: &'static str, label: &'static str) -> Param {
    Param { name, label, unit: "°", degrees: true }
}

const GROUPS: &[(&str, &[Param])] = &[
    (
        "MOTION",
        &[
            p("max_speed", "Max speed", "m/s"),
            p("max_accel", "Max acceleration", "m/s²"),
            p("max_omega", "Max angular speed", "rad/s"),
            p("max_alpha", "Max angular accel.", "rad/s²"),
        ],
    ),
    (
        "WHEELS",
        &[
            deg("front_wheel_angle", "Front wheel angle"),
            deg("back_wheel_angle", "Back wheel angle"),
            p("wheel_distance", "Distance from center", "m"),
            p("wheel_speed", "Wheel top speed (0 = off)", "m/s"),
        ],
    ),
    (
        "TRACTION",
        &[
            p("grip_accel", "Grip (0 = no slip)", "m/s²"),
            p("slide_grip", "Grip while sliding", "0–1"),
            p("slip_spin", "Spin while sliding", "rad/m"),
        ],
    ),
    (
        "KICKER",
        &[
            p("kick_x_speed", "Flat kick speed", "m/s"),
            p("max_kick_speed", "Max kick speed", "m/s"),
            p("kick_reach", "Kick reach", "m"),
        ],
    ),
    (
        "BODY",
        &[
            p("radius", "Radius", "m"),
            p("front_distance", "Front distance", "m"),
            p("body_restitution", "Body bounce", "0–1"),
            p("front_restitution", "Front bounce", "0–1"),
        ],
    ),
];

fn params() -> impl Iterator<Item = &'static Param> {
    GROUPS.iter().flat_map(|(_, ps)| ps.iter())
}

/// Closest the wheels may get to the forward axis, to each other and to the
/// back, degrees.
const MIN_GAP_DEG: f64 = 10.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wheel {
    Front,
    Back,
}

#[derive(Debug, Clone)]
pub enum EditorMessage {
    Edit(&'static str, String),
    /// A wheel dragged on the drawing to this angle from the forward axis, degrees.
    DragWheel(Wheel, f64),
    Name(String),
    /// Reads a saved profile into the editor.
    Open(String),
    /// Sends the edits to the running simulator.
    Apply,
    /// Back to what the simulator runs (or the defaults while it is off).
    Revert,
    Defaults,
    Save,
    RefreshSaved,
    /// First click asks, second click deletes.
    Delete(String),
}

pub struct RobotEditorPanel {
    /// What the simulator runs with; None while it is off.
    running: Option<SimConfig>,
    /// Field text per parameter, in the field's unit.
    typed: BTreeMap<&'static str, String>,
    /// The fields changed since they were last filled from the simulator.
    touched: bool,
    name: String,
    saved: Vec<String>,
    confirm_delete: Option<String>,
    status: Option<Result<String, String>>,
}

impl Default for RobotEditorPanel {
    fn default() -> Self {
        let mut panel = Self {
            running: None,
            typed: BTreeMap::new(),
            touched: false,
            name: "my_robot".to_string(),
            saved: robot_profiles::list(),
            confirm_delete: None,
            status: None,
        };
        panel.fill(&RobotConfig::default());
        panel
    }
}

impl RobotEditorPanel {
    /// The engine sent the simulator's parameters (None = simulator off).
    /// Untouched fields follow the simulator.
    pub fn set_config(&mut self, config: Option<SimConfig>) {
        if let Some(c) = &config {
            if !self.touched || self.draft().ok().as_ref() == Some(&c.robot) {
                self.fill(&c.robot);
                self.touched = false;
            }
        }
        self.running = config;
    }

    fn fill(&mut self, robot: &RobotConfig) {
        self.typed = params()
            .map(|p| {
                let v = robot_profiles::value(robot, p.name).unwrap_or(0.0);
                (p.name, if p.degrees { fmt(v.to_degrees(), 1) } else { fmt(v, 4) })
            })
            .collect();
    }

    /// The robot the fields describe, or what is wrong with them.
    fn draft(&self) -> Result<RobotConfig, String> {
        let mut json = serde_json::Map::new();
        for p in params() {
            let typed = self.typed.get(p.name).map(|s| s.trim()).unwrap_or("");
            let v: f64 = typed.parse().map_err(|_| format!("{} is not a number", p.label))?;
            if !v.is_finite() || v < 0.0 {
                return Err(format!("{} must be a number ≥ 0", p.label));
            }
            json.insert(p.name.to_string(), serde_json::json!(if p.degrees { v.to_radians() } else { v }));
        }
        let robot: RobotConfig = serde_json::from_value(serde_json::Value::Object(json)).map_err(|e| e.to_string())?;
        robot_profiles::check(&robot).map_err(|e| e.replace("robot.", ""))?;
        Ok(robot)
    }

    /// The reference the fields are compared with.
    fn reference(&self) -> RobotConfig {
        self.running.as_ref().map_or_else(RobotConfig::default, |c| c.robot.clone())
    }

    /// Parameters whose field differs from the reference.
    fn changed(&self) -> Vec<&'static str> {
        let reference = self.reference();
        params()
            .filter(|p| {
                let typed = self.typed.get(p.name).and_then(|s| s.trim().parse::<f64>().ok());
                let v = robot_profiles::value(&reference, p.name).unwrap_or(0.0);
                let v = if p.degrees { v.to_degrees() } else { v };
                typed.is_none_or(|t| (t - v).abs() > 1e-9 * v.abs().max(1.0) && fmt(t, 4) != fmt(v, if p.degrees { 1 } else { 4 }))
            })
            .map(|p| p.name)
            .collect()
    }

    /// Handles a message; returns the console commands to run.
    pub fn update(&mut self, message: EditorMessage) -> Vec<String> {
        if !matches!(message, EditorMessage::Delete(_)) {
            self.confirm_delete = None;
        }
        match message {
            EditorMessage::Edit(name, typed) => {
                self.typed.insert(name, typed);
                self.touched = true;
            }
            EditorMessage::DragWheel(wheel, degrees) => {
                let get = |s: &Self, n: &str| s.typed.get(n).and_then(|t| t.trim().parse::<f64>().ok());
                let (front, back) = (get(self, "front_wheel_angle").unwrap_or(60.0), get(self, "back_wheel_angle").unwrap_or(135.0));
                let (name, v) = match wheel {
                    Wheel::Front => ("front_wheel_angle", degrees.clamp(MIN_GAP_DEG, back - MIN_GAP_DEG)),
                    Wheel::Back => ("back_wheel_angle", degrees.clamp(front + MIN_GAP_DEG, 180.0 - MIN_GAP_DEG)),
                };
                self.typed.insert(name, fmt(v.round(), 1));
                self.touched = true;
            }
            EditorMessage::Name(name) => self.name = name,
            EditorMessage::Open(name) => match robot_profiles::load(&name) {
                Ok(robot) => {
                    self.fill(&robot);
                    self.touched = true;
                    self.status = Some(Ok(format!("Opened robots/{name}.toml — Apply to drive with it")));
                    self.name = name;
                }
                Err(e) => self.status = Some(Err(e)),
            },
            EditorMessage::Apply => {
                let Some(running) = &self.running else { return Vec::new() };
                match self.draft() {
                    Ok(robot) => {
                        // Wheel angles in an order the simulator accepts at every step.
                        let mut commands: Vec<(usize, String)> = params()
                            .filter_map(|p| {
                                let v = robot_profiles::value(&robot, p.name)?;
                                (robot_profiles::value(&running.robot, p.name) != Some(v)).then(|| {
                                    let order = match p.name {
                                        "back_wheel_angle" if v > running.robot.back_wheel_angle => 0,
                                        "front_wheel_angle" => 1,
                                        "radius" if v > running.robot.radius => 0,
                                        _ => 2,
                                    };
                                    (order, format!("sim robot {} {v}", p.name))
                                })
                            })
                            .collect();
                        commands.sort_by_key(|c| c.0);
                        self.status = Some(Ok(format!("Applied {} change{}", commands.len(), if commands.len() == 1 { "" } else { "s" })));
                        return commands.into_iter().map(|c| c.1).collect();
                    }
                    Err(e) => self.status = Some(Err(e)),
                }
            }
            EditorMessage::Revert => {
                self.fill(&self.reference());
                self.touched = false;
                self.status = None;
            }
            EditorMessage::Defaults => {
                self.fill(&RobotConfig::default());
                self.touched = true;
            }
            EditorMessage::Save => {
                let name = self.name.trim().to_string();
                self.status = Some(self.draft().and_then(|robot| robot_profiles::save(&name, &robot, "")).map(|path| format!("Saved {}", path.display())));
                self.saved = robot_profiles::list();
            }
            EditorMessage::RefreshSaved => self.saved = robot_profiles::list(),
            EditorMessage::Delete(name) => {
                if self.confirm_delete.as_deref() == Some(&name) {
                    self.confirm_delete = None;
                    self.status = Some(robot_profiles::delete(&name).map(|()| format!("Deleted robots/{name}.toml")));
                    self.saved = robot_profiles::list();
                } else {
                    self.confirm_delete = Some(name);
                }
            }
        }
        Vec::new()
    }

    // --- View ---

    pub fn view(&self) -> Element<'_, EditorMessage> {
        let draft = self.draft();
        // Drawn: the draft, or the last valid robot while a field is wrong.
        let shown = draft.clone().unwrap_or_else(|_| self.reference());
        let changed = self.changed();
        let ball_radius = self.running.as_ref().map_or(ssl_sim::BallConfig::default().radius, |c| c.ball.radius);

        let drawing = card(
            column![
                section_title("TOP VIEW · drag a wheel to change its angle"),
                Canvas::new(Drawing { robot: shown.clone(), ball_radius }).width(Length::Fill).height(Length::Fixed(330.0)),
            ]
            .spacing(6)
            .into(),
        );
        let envelope = card(
            column![
                section_title("SPEED IN EACH DIRECTION"),
                Canvas::new(Envelope { robot: shown.clone() }).width(Length::Fill).height(Length::Fixed(250.0)),
                speed_stats(&shown),
            ]
            .spacing(6)
            .into(),
        );

        let fields = row![
            column![self.group(0, &changed), self.group(1, &changed)].spacing(10).width(Length::Fill),
            column![self.group(2, &changed), self.group(3, &changed), self.group(4, &changed)].spacing(10).width(Length::Fill),
        ]
        .spacing(10);

        let mut content = column![
            self.header(&draft, &changed),
            row![container(drawing).width(Length::FillPortion(11)), container(envelope).width(Length::FillPortion(9))].spacing(10),
            fields,
        ]
        .spacing(10);
        if let Err(e) = &draft {
            content = content.push(status_line(&Err(e.clone())));
        } else if let Some(status) = &self.status {
            content = content.push(status_line(status));
        }
        content = content.push(self.saved_list());
        container(content).padding(10).width(Length::Fill).into()
    }

    fn header(&self, draft: &Result<RobotConfig, String>, changed: &[&'static str]) -> Element<'_, EditorMessage> {
        let n = changed.len();
        let sim_on = self.running.is_some();
        let state: Element<'_, EditorMessage> = match (sim_on, n) {
            (false, _) => pill("SIM off · compared with defaults", theme::MUTED, theme::GROUP),
            (true, 0) => pill("Same as the simulator", theme::GREEN, theme::GREEN_DIM),
            (true, n) => pill(&format!("{n} change{} not applied", if n == 1 { "" } else { "s" }), theme::ACCENT_FG, theme::ACCENT_DIM),
        };
        let valid = draft.is_ok();
        let name_ok = robot_profiles::valid_name(self.name.trim());
        let name = text_input("profile name", &self.name)
            .on_input(EditorMessage::Name)
            .on_submit(EditorMessage::Save)
            .font(theme::MONO)
            .size(13)
            .padding([4, 8])
            .width(Length::Fixed(170.0))
            .style(move |t, s| {
                let mut style = theme::input(t, s);
                if !name_ok {
                    style.border.color = theme::RED;
                }
                style
            });
        let open = pick_list(self.saved.clone(), None::<String>, EditorMessage::Open)
            .placeholder("Open…")
            .text_size(12)
            .padding([4, 8])
            .width(Length::Fixed(130.0));

        let action = |label: &'static str, kind: Btn, message: Option<EditorMessage>| {
            button(text(label).size(12)).padding([5, 12]).style(btn(kind)).on_press_maybe(message)
        };
        card(
            row![
                theme::icon(theme::Icon::Chassis, 18.0, Some(theme::ACCENT)),
                name,
                open,
                state,
                Space::new().width(Length::Fill),
                action("Defaults", Btn::Ghost, Some(EditorMessage::Defaults)),
                action("Revert", Btn::Ghost, (n > 0 || self.touched).then_some(EditorMessage::Revert)),
                action("Apply to SIM", Btn::Toggle(sim_on && valid && n > 0), (sim_on && valid && n > 0).then_some(EditorMessage::Apply)),
                action("Save", Btn::Play, (valid && name_ok).then_some(EditorMessage::Save)),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .into(),
        )
    }

    fn group(&self, index: usize, changed: &[&'static str]) -> Element<'_, EditorMessage> {
        let (title, ps) = GROUPS[index];
        let mut rows = column![section_title(title)].spacing(3);
        for p in ps {
            let typed = self.typed.get(p.name).cloned().unwrap_or_default();
            let edited = changed.contains(&p.name);
            let invalid = typed.trim().parse::<f64>().map_or(true, |v| !v.is_finite() || v < 0.0);
            let name = p.name;
            let input = text_input("", &typed)
                .on_input(move |s| EditorMessage::Edit(name, s))
                .on_submit(EditorMessage::Apply)
                .font(theme::MONO)
                .size(12)
                .padding([2, 6])
                .align_x(Horizontal::Right)
                .width(Length::Fixed(72.0))
                .style(move |t, s| {
                    let mut style = theme::input(t, s);
                    if invalid {
                        style.border.color = theme::RED;
                        style.value = theme::RED_FG;
                    } else if edited {
                        style.border.color = theme::ACCENT;
                        style.value = theme::ACCENT_FG;
                    }
                    style
                });
            rows = rows.push(
                row![
                    text(p.label).size(12).color(if edited { theme::TEXT } else { theme::TEXT_2 }).width(Length::Fill),
                    input,
                    text(p.unit).size(11).font(theme::MONO).color(theme::MUTED).width(Length::Fixed(42.0)),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            );
        }
        card(rows.into())
    }

    fn saved_list(&self) -> Element<'_, EditorMessage> {
        let mut list = row![].spacing(6).align_y(Alignment::Center);
        if self.saved.is_empty() {
            list = list.push(text("None yet. Save writes robots/<name>.toml; load one in the console with `sim robot load <name>`.").size(11).color(theme::MUTED));
        }
        for name in &self.saved {
            let confirming = self.confirm_delete.as_deref() == Some(name.as_str());
            list = list.push(
                container(
                    row![
                        button(text(name.clone()).size(11).font(theme::MONO)).padding([2, 6]).style(btn(Btn::Ghost)).on_press(EditorMessage::Open(name.clone())),
                        button(text(if confirming { "delete?" } else { "×" }).size(11))
                            .padding([2, 6])
                            .style(btn(if confirming { Btn::Rec(true) } else { Btn::Ghost }))
                            .on_press(EditorMessage::Delete(name.clone())),
                    ]
                    .align_y(Alignment::Center),
                )
                .style(theme::group),
            );
        }
        card(
            column![
                row![
                    section_title("SAVED PROFILES · robots/"),
                    Space::new().width(Length::Fill),
                    button(text("Reload").size(11)).padding([2, 8]).style(btn(Btn::Ghost)).on_press(EditorMessage::RefreshSaved),
                ]
                .align_y(Alignment::Center),
                list.wrap(),
            ]
            .spacing(6)
            .into(),
        )
    }
}

// --- Top view ---

/// The robot to scale, facing up, with draggable wheels.
struct Drawing {
    robot: RobotConfig,
    ball_radius: f64,
}

#[derive(Default)]
struct DragState {
    dragging: Option<Wheel>,
}

/// How the drawing maps meters to the screen: forward is up.
struct View {
    center: Point,
    scale: f32,
}

impl View {
    fn new(bounds: Rectangle, robot: &RobotConfig, ball_radius: f64) -> Self {
        let up = robot.radius.max(robot.front_distance + 2.0 * ball_radius + robot.kick_reach) + 0.02;
        let down = robot.radius + 0.02;
        let side = robot.radius + 0.03;
        let scale = (bounds.height as f64 / (up + down)).min(bounds.width as f64 / (2.0 * side)) as f32;
        Self { center: Point::new(bounds.width / 2.0, up as f32 * scale + (bounds.height - (up + down) as f32 * scale) / 2.0), scale }
    }

    /// A point in the robot's frame (x forward, y left), m.
    fn at(&self, x: f64, y: f64) -> Point {
        Point::new(self.center.x - y as f32 * self.scale, self.center.y - x as f32 * self.scale)
    }

    /// Unit screen vector for a robot-frame angle (0 = forward, CCW).
    fn dir(phi: f64) -> Vector {
        Vector::new(-phi.sin() as f32, -phi.cos() as f32)
    }

    /// Screen angle (for arcs) of a robot-frame angle.
    fn screen_angle(phi: f64) -> f32 {
        let d = Self::dir(phi);
        d.y.atan2(d.x)
    }
}

impl canvas::Program<EditorMessage> for Drawing {
    type State = DragState;

    fn update(&self, state: &mut DragState, event: &canvas::Event, bounds: Rectangle, cursor: mouse::Cursor) -> Option<canvas::Action<EditorMessage>> {
        let view = View::new(bounds, &self.robot, self.ball_radius);
        match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let pos = cursor.position_in(bounds)?;
                let grab = (0.025 * view.scale).max(12.0);
                let wheel = self.robot.wheel_angles().iter().zip([Wheel::Front, Wheel::Back, Wheel::Back, Wheel::Front]).find_map(|(&phi, w)| {
                    let c = view.at(self.robot.wheel_distance * phi.cos(), self.robot.wheel_distance * phi.sin());
                    (c.distance(pos) <= grab).then_some(w)
                })?;
                state.dragging = Some(wheel);
                Some(canvas::Action::request_redraw().and_capture())
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let wheel = state.dragging?;
                let pos = cursor.position_in(bounds)?;
                let (dx, dy) = ((pos.x - view.center.x) as f64, (pos.y - view.center.y) as f64);
                let degrees = (-dx).atan2(-dy).abs().to_degrees();
                Some(canvas::Action::publish(EditorMessage::DragWheel(wheel, degrees)).and_capture())
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                state.dragging.take()?;
                Some(canvas::Action::request_redraw().and_capture())
            }
            _ => None,
        }
    }

    fn mouse_interaction(&self, state: &DragState, bounds: Rectangle, cursor: mouse::Cursor) -> mouse::Interaction {
        if state.dragging.is_some() {
            return mouse::Interaction::Grabbing;
        }
        let Some(pos) = cursor.position_in(bounds) else { return mouse::Interaction::default() };
        let view = View::new(bounds, &self.robot, self.ball_radius);
        let grab = (0.025 * view.scale).max(12.0);
        let near = self.robot.wheel_angles().iter().any(|&phi| {
            view.at(self.robot.wheel_distance * phi.cos(), self.robot.wheel_distance * phi.sin()).distance(pos) <= grab
        });
        if near { mouse::Interaction::Grab } else { mouse::Interaction::default() }
    }

    fn draw(&self, state: &DragState, renderer: &Renderer, _: &Theme, bounds: Rectangle, cursor: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let r = &self.robot;
        let view = View::new(bounds, r, self.ball_radius);
        let s = view.scale;
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), theme::FIELD_FLOOR);

        // 1 cm grid, every 5 cm stronger, aligned on the center.
        let cm = 0.01 * s;
        if cm >= 4.0 {
            let lines = |frame: &mut Frame, from: f32, len: f32, vertical: bool| {
                let lo = (-from / cm).floor() as i32;
                let hi = ((len - from) / cm).ceil() as i32;
                for k in lo..=hi {
                    let c = from + k as f32 * cm;
                    let color = Color { a: if k % 5 == 0 { 0.09 } else { 0.035 }, ..theme::FIELD_LINE };
                    let (a, b) = if vertical {
                        (Point::new(c, 0.0), Point::new(c, bounds.height))
                    } else {
                        (Point::new(0.0, c), Point::new(bounds.width, c))
                    };
                    frame.stroke(&Path::line(a, b), Stroke::default().with_color(color).with_width(1.0));
                }
            };
            lines(&mut frame, view.center.x, bounds.width, true);
            lines(&mut frame, view.center.y, bounds.height, false);
        }

        // Kick zone and ball in front of the face.
        let half_face = r.front_half_width();
        let zone = view.at(r.front_distance + r.kick_reach, half_face);
        frame.fill_rectangle(zone, Size::new((2.0 * half_face) as f32 * s, (r.kick_reach as f32 * s).max(1.0)), Color { a: 0.35, ..theme::ORANGE });
        frame.fill(&Path::circle(view.at(r.front_distance + self.ball_radius, 0.0), self.ball_radius as f32 * s), theme::BALL);

        // Body: a circle cut flat at the front.
        let cut = (r.front_distance / r.radius).clamp(-1.0, 1.0).acos() as f32;
        let up = -std::f32::consts::FRAC_PI_2;
        let body = Path::new(|b| {
            b.arc(Arc {
                center: view.center,
                radius: r.radius as f32 * s,
                start_angle: iced::Radians(up + cut),
                end_angle: iced::Radians(up + std::f32::consts::TAU - cut),
            });
            b.close();
        });
        frame.fill(&body, Color { a: 0.55, ..theme::SSL_BLUE });
        frame.stroke(&body, Stroke::default().with_color(theme::SSL_BLUE).with_width(1.5));
        frame.stroke(
            &Path::line(view.at(r.front_distance, half_face), view.at(r.front_distance, -half_face)),
            Stroke::default().with_color(theme::TEXT).with_width(3.0),
        );

        // Forward axis.
        let dashed = |c: Color| Stroke {
            line_dash: LineDash { segments: &[4.0, 4.0], offset: 0 },
            ..Stroke::default().with_color(c).with_width(1.0)
        };
        frame.stroke(&Path::line(view.center, view.at(r.front_distance, 0.0)), dashed(Color { a: 0.5, ..theme::TEXT }));

        // Wheels: axle from the center, the wheel across it (it rolls at right
        // angles to its axle).
        let hovered = cursor.position_in(bounds);
        let wheel_len = (0.028 * s).max(10.0);
        let wheel_w = (0.011 * s).max(5.0);
        for (&phi, which) in r.wheel_angles().iter().zip([Wheel::Front, Wheel::Back, Wheel::Back, Wheel::Front]) {
            let c = view.at(r.wheel_distance * phi.cos(), r.wheel_distance * phi.sin());
            let roll = View::dir(phi + PI / 2.0);
            let active = state.dragging == Some(which)
                || (state.dragging.is_none() && hovered.is_some_and(|h| h.distance(c) <= (0.025 * s).max(12.0)));
            frame.stroke(&Path::line(view.center, c), Stroke::default().with_color(Color { a: 0.35, ..theme::TEXT }).with_width(1.0));
            let wheel = Path::line(c - roll * (wheel_len / 2.0), c + roll * (wheel_len / 2.0));
            let color = if active { theme::ACCENT } else { theme::TEXT_2 };
            frame.stroke(&wheel, Stroke { line_cap: canvas::LineCap::Round, ..Stroke::default().with_color(color).with_width(wheel_w) });
            // Rollers.
            for k in [-1.0, 0.0, 1.0] {
                let p = c + roll * (k * wheel_len / 3.2);
                let across = View::dir(phi) * (wheel_w / 2.0);
                frame.stroke(&Path::line(p - across, p + across), Stroke::default().with_color(theme::FIELD_FLOOR).with_width(1.0));
            }
        }

        // Angle arcs on the right-hand wheels, labelled.
        for (phi, radius, color) in [(r.front_wheel_angle, 0.45, theme::ACCENT_FG), (r.back_wheel_angle, 0.7, theme::TEXT_2)] {
            let rad = (r.wheel_distance * radius) as f32 * s;
            let arc = Path::new(|b| {
                b.arc(Arc {
                    center: view.center,
                    radius: rad,
                    start_angle: iced::Radians(View::screen_angle(0.0)),
                    end_angle: iced::Radians(View::screen_angle(-phi)),
                });
            });
            frame.stroke(&arc, Stroke::default().with_color(color).with_width(1.2));
            let mid = view.center + View::dir(-phi / 2.0) * (rad + 14.0);
            frame.fill_text(Text {
                content: format!("{:.0}°", phi.to_degrees()),
                position: mid,
                color,
                size: iced::Pixels(11.0),
                font: theme::MONO_BOLD,
                align_x: Horizontal::Center.into(),
                align_y: Vertical::Center.into(),
                ..Text::default()
            });
        }

        let label = |frame: &mut Frame, content: String, x: f32, align: Horizontal| {
            frame.fill_text(Text {
                content,
                position: Point::new(x, bounds.height - 10.0),
                color: theme::MUTED,
                size: iced::Pixels(10.0),
                font: theme::MONO,
                align_x: align.into(),
                align_y: Vertical::Center.into(),
                ..Text::default()
            });
        };
        label(&mut frame, format!("Ø {:.0} mm · face {:.0} mm", r.radius * 2000.0, half_face * 2000.0), 8.0, Horizontal::Left);
        label(&mut frame, if cm >= 4.0 { "grid 1 cm".into() } else { String::new() }, bounds.width - 8.0, Horizontal::Right);

        vec![frame.into_geometry()]
    }
}

// --- Speed envelope ---

/// Top speed in every direction (forward up), against the max speed circle.
struct Envelope {
    robot: RobotConfig,
}

impl<M> canvas::Program<M> for Envelope {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let r = &self.robot;
        let center = Point::new(bounds.width / 2.0, bounds.height / 2.0);
        let radius = (bounds.width.min(bounds.height) / 2.0 - 22.0).max(10.0);
        let max = r.max_speed.max(1e-6);
        let to = |phi: f64, v: f64| center + View::dir(phi) * ((v / max) as f32 * radius);

        // Rings every 1 m/s and spokes every 45°.
        let faint = Stroke::default().with_color(Color { a: 0.07, ..theme::FIELD_LINE }).with_width(1.0);
        let mut v = 1.0;
        while v < max {
            frame.stroke(&Path::circle(center, (v / max) as f32 * radius), faint);
            v += 1.0;
        }
        for k in 0..8 {
            frame.stroke(&Path::line(center, to(k as f64 * PI / 4.0, max)), faint);
        }
        frame.stroke(
            &Path::circle(center, radius),
            Stroke { line_dash: LineDash { segments: &[4.0, 4.0], offset: 0 }, ..Stroke::default().with_color(theme::MUTED).with_width(1.0) },
        );

        // The envelope.
        let shape = Path::new(|b| {
            for k in 0..=360 {
                let phi = (k as f64).to_radians();
                let p = to(phi, r.top_speed_towards(phi));
                if k == 0 { b.move_to(p) } else { b.line_to(p) }
            }
            b.close();
        });
        frame.fill(&shape, Color { a: 0.22, ..theme::ACCENT });
        frame.stroke(&shape, Stroke::default().with_color(theme::ACCENT).with_width(1.8));

        // Wheels' rolling directions as ticks on the rim.
        for phi in r.wheel_angles() {
            for side in [PI / 2.0, -PI / 2.0] {
                let a = phi + side;
                frame.stroke(&Path::line(to(a, max * 1.02), to(a, max * 1.1)), Stroke::default().with_color(theme::TEXT_2).with_width(2.0));
            }
        }

        // Small robot in the middle, facing up.
        let tri = Path::new(|b| {
            b.move_to(center + Vector::new(0.0, -7.0));
            b.line_to(center + Vector::new(5.0, 5.0));
            b.line_to(center + Vector::new(-5.0, 5.0));
            b.close();
        });
        frame.fill(&tri, theme::SSL_BLUE);

        let label = |frame: &mut Frame, content: String, p: Point| {
            frame.fill_text(Text {
                content,
                position: p,
                color: theme::MUTED,
                size: iced::Pixels(10.0),
                font: theme::MONO,
                align_x: Horizontal::Center.into(),
                align_y: Vertical::Center.into(),
                ..Text::default()
            });
        };
        label(&mut frame, "forward".into(), Point::new(center.x, center.y - radius - 12.0));
        label(&mut frame, format!("max {} m/s", fmt(max, 3)), Point::new(center.x, center.y + radius + 12.0));

        vec![frame.into_geometry()]
    }
}

fn speed_stats<'a>(r: &RobotConfig) -> Element<'a, EditorMessage> {
    let (slowest, at) = (0..360).map(|k| (r.top_speed_towards((k as f64).to_radians()), k)).fold((f64::MAX, 0), |m, x| if x.0 < m.0 { x } else { m });
    let spin = if r.wheel_speed > 0.0 { (r.wheel_speed / r.wheel_distance).min(r.max_omega) } else { r.max_omega };
    let stat = |label: &'static str, value: String| {
        column![text(label).size(10).color(theme::MUTED), text(value).size(13).font(theme::MONO_BOLD).color(theme::TEXT)]
            .spacing(1)
            .width(Length::Fill)
            .align_x(Horizontal::Center)
    };
    let mut col = column![row![
        stat("forward", format!("{}", fmt(r.top_speed_towards(0.0), 3))),
        stat("sideways", format!("{}", fmt(r.top_speed_towards(PI / 2.0), 3))),
        stat("slowest", format!("{} @{at}°", fmt(slowest, 3))),
        stat("turn rad/s", fmt(spin, 3)),
    ]]
    .spacing(4);
    if r.wheel_speed == 0.0 {
        col = col.push(
            text("Wheel top speed is 0 (off): the robot reaches max speed every way. Give the wheels a top speed to see what their angles do.")
                .size(10)
                .color(theme::ORANGE),
        );
    }
    col.into()
}

// --- Pieces ---

fn pill<'a>(label: &str, fg: Color, bg: Color) -> Element<'a, EditorMessage> {
    container(text(label.to_string()).size(11).color(fg))
        .padding([3, 10])
        .style(move |_| container::Style {
            background: Some(Background::Color(bg)),
            border: Border { radius: 10.0.into(), ..Default::default() },
            ..Default::default()
        })
        .into()
}

fn status_line<'a>(status: &Result<String, String>) -> Element<'a, EditorMessage> {
    let (msg, fg, bg) = match status {
        Ok(m) => (m.clone(), theme::GREEN, theme::GREEN_DIM),
        Err(e) => (e.clone(), theme::RED_FG, theme::RED_DIM),
    };
    container(text(msg).size(11).color(fg))
        .padding([5, 10])
        .width(Length::Fill)
        .style(move |_| container::Style {
            background: Some(Background::Color(bg)),
            border: Border { radius: 4.0.into(), ..Default::default() },
            ..Default::default()
        })
        .into()
}

fn section_title(title: &str) -> Element<'_, EditorMessage> {
    text(title).size(11).font(theme::MONO_BOLD).color(theme::MUTED).into()
}

fn card(content: Element<'_, EditorMessage>) -> Element<'_, EditorMessage> {
    container(content).padding([10, 12]).width(Length::Fill).style(theme::group).into()
}

/// At most `decimals` decimals, no trailing zeros.
fn fmt(v: f64, decimals: usize) -> String {
    let s = format!("{v:.decimals$}");
    if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.').to_string() } else { s }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_robot_parameter_has_a_field() {
        let robot = RobotConfig::default();
        for p in params() {
            assert!(robot_profiles::value(&robot, p.name).is_some(), "{}", p.name);
        }
        assert_eq!(params().count(), serde_json::to_value(&robot).unwrap().as_object().unwrap().len());
    }

    #[test]
    fn fields_round_trip_with_angles_in_degrees() {
        let panel = RobotEditorPanel::default();
        assert_eq!(panel.typed["front_wheel_angle"], "60");
        assert_eq!(panel.draft().unwrap(), RobotConfig::default());
    }

    #[test]
    fn dragging_a_wheel_keeps_the_angles_apart() {
        let mut panel = RobotEditorPanel::default();
        panel.update(EditorMessage::DragWheel(Wheel::Front, 170.0));
        assert_eq!(panel.typed["front_wheel_angle"], "125"); // back is 135°
        panel.update(EditorMessage::DragWheel(Wheel::Back, 2.0));
        assert_eq!(panel.typed["back_wheel_angle"], "135"); // front + 10°
        assert!(panel.draft().is_ok());
    }

    #[test]
    fn apply_sends_only_the_changes() {
        let mut panel = RobotEditorPanel::default();
        panel.set_config(Some(SimConfig::default()));
        assert!(panel.changed().is_empty());
        panel.update(EditorMessage::Edit("max_speed", "2.5".into()));
        panel.update(EditorMessage::DragWheel(Wheel::Front, 45.0));
        let commands = panel.update(EditorMessage::Apply);
        assert_eq!(commands.len(), 2, "{commands:?}");
        assert!(commands.contains(&"sim robot max_speed 2.5".to_string()));
        assert!(commands.iter().any(|c| c.starts_with("sim robot front_wheel_angle 0.785")));
    }

    #[test]
    fn invalid_fields_block_apply() {
        let mut panel = RobotEditorPanel::default();
        panel.set_config(Some(SimConfig::default()));
        panel.update(EditorMessage::Edit("slide_grip", "1.5".into()));
        assert!(panel.update(EditorMessage::Apply).is_empty());
        assert!(matches!(&panel.status, Some(Err(e)) if e.contains("slide_grip")));
    }
}
