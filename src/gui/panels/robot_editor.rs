// gui/panels/robot_editor.rs — Edit the simulator's robot and save it as a profile
//
// One robot, three views of it:
// - a top-down drawing to scale (body, kicker, kick zone, ball, the four omni
//   wheels drawn by their grip); drag a wheel around the body to move it,
//   alone or mirrored with its partner on the other side;
// - its envelope: how fast it can drive, or accelerate, in each direction,
//   which the wheels' angles, top speeds and torques shape;
// - every parameter as a field (one column per wheel), highlighted while it
//   differs from what the simulator runs.
// Apply sends every change in one `sim robot set ...` line, checked as a
// whole, and confirms once the simulator reports it runs the new robot.
// Save writes robots/<name>.toml (robot_profiles), Open reads one back.

use iced::alignment::{Horizontal, Vertical};
use iced::mouse;
use iced::widget::canvas::{self, path::Arc, Canvas, Frame, Geometry, LineDash, Path, Stroke, Text};
use iced::widget::{button, checkbox, column, container, pick_list, row, text, text_input, Space};
use iced::{Alignment, Background, Border, Color, Element, Length, Point, Rectangle, Renderer, Size, Theme, Vector};
use ssl_sim::{RobotConfig, SimConfig};
use std::collections::BTreeMap;
use std::f64::consts::PI;

use crate::gui::theme::{self, btn, Btn};
use crate::robot_profiles;

/// One robot-wide parameter: config name, label, unit.
struct Param {
    name: &'static str,
    label: &'static str,
    unit: &'static str,
}

const fn p(name: &'static str, label: &'static str, unit: &'static str) -> Param {
    Param { name, label, unit }
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
        "TRACTION",
        &[
            p("grip_accel", "Floor grip (0 = no slip)", "m/s²"),
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
            p("mass", "Mass", "kg"),
            p("body_restitution", "Body bounce", "0–1"),
            p("front_restitution", "Front bounce", "0–1"),
        ],
    ),
];

/// Wheel geometry shared by the four wheels.
const WHEEL_SHARED: &[Param] = &[p("wheel_distance", "Distance from center", "m"), p("wheel_radius", "Wheel radius", "m")];

/// Per-wheel fields: name in `WheelConfig`, row label, unit.
const WHEEL_FIELDS: &[(&str, &str, &str)] = &[
    ("angle", "Angle", "°"),
    ("max_speed", "Top speed", "m/s"),
    ("torque", "Torque", "N·m"),
    ("grip", "Grip", "×"),
];

const WHEEL_NAMES: [&str; 4] = ["FL", "BL", "BR", "FR"];
const WHEEL_LONG: [&str; 4] = ["front left", "back left", "back right", "front right"];

/// The wheel on the other side (front left ↔ front right, back left ↔ back right).
fn partner(i: usize) -> usize {
    3 - i
}

fn wheel_key(i: usize, field: &str) -> String {
    format!("wheels.{i}.{field}")
}

/// Every field key: robot-wide names, then `wheels.<i>.<field>`.
fn keys() -> Vec<String> {
    let mut keys: Vec<String> = GROUPS.iter().flat_map(|(_, ps)| ps.iter()).chain(WHEEL_SHARED).map(|p| p.name.to_string()).collect();
    for i in 0..4 {
        keys.extend(WHEEL_FIELDS.iter().map(|(f, ..)| wheel_key(i, f)));
    }
    keys
}

fn is_angle(key: &str) -> bool {
    key.ends_with(".angle")
}

/// A parameter's value by key, in config units (rad for angles).
fn get(robot: &RobotConfig, key: &str) -> Option<f64> {
    let mut v = serde_json::to_value(robot).ok()?;
    for part in key.split('.') {
        v = match v {
            serde_json::Value::Object(mut m) => m.remove(part)?,
            serde_json::Value::Array(mut a) => {
                let i: usize = part.parse().ok()?;
                (i < a.len()).then(|| a.swap_remove(i))?
            }
            _ => return None,
        };
    }
    v.as_f64()
}

/// The text a field shows for a config value.
fn field_text(key: &str, v: f64) -> String {
    if is_angle(key) { fmt(v.to_degrees(), 1) } else { fmt(v, 4) }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Plot {
    Speed,
    Accel,
}

#[derive(Debug, Clone)]
pub enum EditorMessage {
    Edit(String, String),
    /// Wheel i dragged on the drawing to this angle, degrees (CCW from forward).
    DragWheel(usize, f64),
    SelectWheel(usize),
    Mirror(bool),
    Plot(Plot),
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
    /// Field text per key, in the field's unit (degrees for angles).
    typed: BTreeMap<String, String>,
    /// The fields changed since they were last filled from the simulator.
    touched: bool,
    /// Sent with Apply, waiting for the simulator to report it.
    pending: Option<RobotConfig>,
    selected_wheel: usize,
    /// Dragging or editing a wheel changes its partner too, mirrored.
    mirror: bool,
    plot: Plot,
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
            pending: None,
            selected_wheel: 0,
            mirror: true,
            plot: Plot::Speed,
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
    /// Untouched fields follow the simulator; a pending Apply is confirmed
    /// or reported as rejected.
    pub fn set_config(&mut self, config: Option<SimConfig>) {
        if let Some(c) = &config {
            if let Some(pending) = self.pending.take() {
                self.status = Some(if c.robot == pending {
                    Ok("The simulator now runs this robot".to_string())
                } else {
                    Err("The simulator did not take the change: see the Lua console".to_string())
                });
            }
            if !self.touched || self.draft().ok().as_ref() == Some(&c.robot) {
                self.fill(&c.robot);
                self.touched = false;
            }
        } else {
            self.pending = None;
        }
        self.running = config;
    }

    fn fill(&mut self, robot: &RobotConfig) {
        self.typed = keys()
            .into_iter()
            .map(|k| {
                let t = field_text(&k, get(robot, &k).unwrap_or(0.0));
                (k, t)
            })
            .collect();
    }

    fn typed_value(&self, key: &str) -> Option<f64> {
        self.typed.get(key).and_then(|s| s.trim().parse::<f64>().ok())
    }

    /// The robot the fields describe, or what is wrong with them.
    fn draft(&self) -> Result<RobotConfig, String> {
        let mut robot = serde_json::to_value(RobotConfig::default()).map_err(|e| e.to_string())?;
        for key in keys() {
            let label = label_of(&key);
            let v = self.typed_value(&key).filter(|v| v.is_finite()).ok_or(format!("{label} is not a number"))?;
            if !is_angle(&key) && v < 0.0 {
                return Err(format!("{label} must be ≥ 0"));
            }
            let v = if is_angle(&key) { v.to_radians() } else { v };
            let mut slot = &mut robot;
            for part in key.split('.') {
                slot = match part.parse::<usize>() {
                    Ok(i) => &mut slot[i],
                    Err(_) => &mut slot[part],
                };
            }
            *slot = serde_json::json!(v);
        }
        let robot: RobotConfig = serde_json::from_value(robot).map_err(|e| e.to_string())?;
        robot_profiles::check(&robot).map_err(|e| e.replace("robot.", ""))?;
        Ok(robot)
    }

    /// The reference the fields are compared with.
    fn reference(&self) -> RobotConfig {
        self.running.as_ref().map_or_else(RobotConfig::default, |c| c.robot.clone())
    }

    /// Keys whose field differs from the reference.
    fn changed(&self) -> Vec<String> {
        let reference = self.reference();
        keys()
            .into_iter()
            .filter(|k| {
                let shown = field_text(k, get(&reference, k).unwrap_or(0.0));
                match self.typed_value(k) {
                    Some(t) => fmt(t, if is_angle(k) { 1 } else { 4 }) != shown,
                    None => true,
                }
            })
            .collect()
    }

    fn set_angle(&mut self, i: usize, degrees: f64) {
        if self.mirror {
            // Left wheels (0, 1) stay on the left, their partners on the right.
            let magnitude = degrees.abs().clamp(5.0, 175.0).round();
            let own = if i < 2 { magnitude } else { -magnitude };
            self.typed.insert(wheel_key(i, "angle"), fmt(own, 1));
            self.typed.insert(wheel_key(partner(i), "angle"), fmt(-own, 1));
        } else {
            self.typed.insert(wheel_key(i, "angle"), fmt(degrees.clamp(-180.0, 180.0).round(), 1));
        }
        self.touched = true;
    }

    /// Handles a message; returns the console commands to run.
    pub fn update(&mut self, message: EditorMessage) -> Vec<String> {
        if !matches!(message, EditorMessage::Delete(_)) {
            self.confirm_delete = None;
        }
        match message {
            EditorMessage::Edit(key, typed) => {
                // Mirrored wheels: the partner follows (its angle negated).
                if self.mirror {
                    if let Some((i, field)) = parse_wheel_key(&key) {
                        let mirrored = if field == "angle" {
                            typed.trim().parse::<f64>().map_or(typed.clone(), |v| fmt(-v, 1))
                        } else {
                            typed.clone()
                        };
                        self.typed.insert(wheel_key(partner(i), field), mirrored);
                    }
                }
                self.typed.insert(key, typed);
                self.touched = true;
            }
            EditorMessage::DragWheel(i, degrees) => {
                self.selected_wheel = i;
                self.set_angle(i, degrees);
            }
            EditorMessage::SelectWheel(i) => self.selected_wheel = i,
            EditorMessage::Mirror(on) => self.mirror = on,
            EditorMessage::Plot(plot) => self.plot = plot,
            EditorMessage::Name(name) => self.name = name,
            EditorMessage::Open(name) => match robot_profiles::load(&name) {
                Ok(robot) => {
                    self.fill(&robot);
                    self.touched = true;
                    self.status = Some(Ok(format!("Opened robots/{name}.toml: Apply to drive with it")));
                    self.name = name;
                }
                Err(e) => self.status = Some(Err(e)),
            },
            EditorMessage::Apply => {
                let Some(running) = &self.running else { return Vec::new() };
                match self.draft() {
                    Ok(robot) => {
                        let pairs: Vec<String> = keys()
                            .into_iter()
                            .filter_map(|k| {
                                let v = get(&robot, &k)?;
                                (get(&running.robot, &k) != Some(v)).then(|| format!("{k} {v}"))
                            })
                            .collect();
                        if pairs.is_empty() {
                            return Vec::new();
                        }
                        self.pending = Some(robot);
                        self.status = Some(Ok("Applying…".to_string()));
                        return vec![format!("sim robot set {}", pairs.join(" "))];
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
                row![
                    section_title("TOP VIEW · drag a wheel"),
                    Space::new().width(Length::Fill),
                    checkbox(self.mirror).label("Mirror left/right").on_toggle(EditorMessage::Mirror).style(theme::check).size(13).text_size(11),
                ]
                .align_y(Alignment::Center),
                Canvas::new(Drawing { robot: shown.clone(), ball_radius, selected: self.selected_wheel })
                    .width(Length::Fill)
                    .height(Length::Fixed(330.0)),
            ]
            .spacing(6)
            .into(),
        );

        let tab = |label: &'static str, plot: Plot| {
            button(text(label).size(11)).padding([2, 8]).style(btn(Btn::Toggle(self.plot == plot))).on_press(EditorMessage::Plot(plot))
        };
        let envelope = card(
            column![
                row![
                    section_title(match self.plot {
                        Plot::Speed => "TOP SPEED BY DIRECTION",
                        Plot::Accel => "ACCELERATION BY DIRECTION",
                    }),
                    Space::new().width(Length::Fill),
                    tab("Speed", Plot::Speed),
                    tab("Accel.", Plot::Accel),
                ]
                .spacing(4)
                .align_y(Alignment::Center),
                Canvas::new(Envelope { robot: shown.clone(), plot: self.plot }).width(Length::Fill).height(Length::Fixed(250.0)),
                envelope_stats(&shown, self.plot),
            ]
            .spacing(6)
            .into(),
        );

        let fields = row![
            column![self.wheels_card(&shown, &changed), self.group(0, &changed)].spacing(10).width(Length::Fill),
            column![self.group(1, &changed), self.group(2, &changed), self.group(3, &changed)].spacing(10).width(Length::Fill),
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

    fn header(&self, draft: &Result<RobotConfig, String>, changed: &[String]) -> Element<'_, EditorMessage> {
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
        let can_apply = sim_on && valid && n > 0;
        card(
            row![
                theme::icon(theme::Icon::Chassis, 18.0, Some(theme::ACCENT)),
                name,
                open,
                state,
                Space::new().width(Length::Fill),
                action("Defaults", Btn::Ghost, Some(EditorMessage::Defaults)),
                action("Revert", Btn::Ghost, (n > 0 || self.touched).then_some(EditorMessage::Revert)),
                action("Apply to SIM", Btn::Toggle(can_apply), can_apply.then_some(EditorMessage::Apply)),
                action("Save", Btn::Play, (valid && name_ok).then_some(EditorMessage::Save)),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .into(),
        )
    }

    /// A text field for `key`, highlighted when edited or invalid.
    fn field(&self, key: String, changed: &[String], width: f32) -> Element<'_, EditorMessage> {
        let typed = self.typed.get(&key).cloned().unwrap_or_default();
        let edited = changed.contains(&key);
        let invalid = typed.trim().parse::<f64>().map_or(true, |v| !v.is_finite() || (!is_angle(&key) && v < 0.0));
        text_input("", &typed)
            .on_input(move |s| EditorMessage::Edit(key.clone(), s))
            .on_submit(EditorMessage::Apply)
            .font(theme::MONO)
            .size(12)
            .padding([2, 6])
            .align_x(Horizontal::Right)
            .width(Length::Fixed(width))
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
            })
            .into()
    }

    fn param_row(&self, p: &'static Param, changed: &[String]) -> Element<'_, EditorMessage> {
        let edited = changed.iter().any(|c| c == p.name);
        row![
            text(p.label).size(12).color(if edited { theme::TEXT } else { theme::TEXT_2 }).width(Length::Fill),
            self.field(p.name.to_string(), changed, 72.0),
            text(p.unit).size(11).font(theme::MONO).color(theme::MUTED).width(Length::Fixed(42.0)),
        ]
        .spacing(6)
        .align_y(Alignment::Center)
        .into()
    }

    fn group(&self, index: usize, changed: &[String]) -> Element<'_, EditorMessage> {
        let (title, ps) = GROUPS[index];
        let mut rows = column![section_title(title)].spacing(3);
        for p in ps {
            rows = rows.push(self.param_row(p, changed));
        }
        card(rows.into())
    }

    /// One column per wheel, plus the shared wheel geometry.
    fn wheels_card(&self, robot: &RobotConfig, changed: &[String]) -> Element<'_, EditorMessage> {
        const COL: f32 = 62.0;
        const LABEL: f32 = 84.0;
        let mut head = row![Space::new().width(Length::Fixed(LABEL))].spacing(4).align_y(Alignment::Center);
        for (i, name) in WHEEL_NAMES.iter().enumerate() {
            let sel = i == self.selected_wheel;
            head = head.push(
                button(text(*name).size(11).font(theme::MONO_BOLD).width(Length::Fill).align_x(Horizontal::Center))
                    .width(Length::Fixed(COL))
                    .padding([2, 0])
                    .style(btn(if sel { Btn::Toggle(true) } else { Btn::Ghost }))
                    .on_press(EditorMessage::SelectWheel(i)),
            );
        }
        let mut table = column![head].spacing(3);
        for (field, label, unit) in WHEEL_FIELDS {
            let mut line = row![text(format!("{label} {unit}")).size(11).color(theme::TEXT_2).width(Length::Fixed(LABEL))].spacing(4).align_y(Alignment::Center);
            for i in 0..4 {
                line = line.push(self.field(wheel_key(i, field), changed, COL));
            }
            table = table.push(line);
        }
        // What the motor torques give: the acceleration each wheel can push.
        let mut accel = row![text("Push m/s²").size(11).color(theme::MUTED).width(Length::Fixed(LABEL))].spacing(4).align_y(Alignment::Center);
        for i in 0..4 {
            let a = robot.wheel_max_accel(i);
            accel = accel.push(
                text(if a.is_finite() { fmt(a, 2) } else { "∞".into() })
                    .size(11)
                    .font(theme::MONO)
                    .color(theme::MUTED)
                    .width(Length::Fixed(COL))
                    .align_x(Horizontal::Right),
            );
        }
        table = table.push(accel);

        let mut shared = column![].spacing(3);
        for p in WHEEL_SHARED {
            shared = shared.push(self.param_row(p, changed));
        }
        card(
            column![
                row![
                    section_title("WHEELS"),
                    Space::new().width(Length::Fill),
                    text(format!("selected: {}", WHEEL_LONG[self.selected_wheel])).size(10).color(theme::MUTED),
                ]
                .align_y(Alignment::Center),
                table,
                shared,
                text("Top speed and torque 0 = no limit. Grip is each wheel's share of the floor grip (1 = normal); uneven grip turns the robot while it slides.")
                    .size(10)
                    .color(theme::MUTED),
            ]
            .spacing(6)
            .into(),
        )
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

fn parse_wheel_key(key: &str) -> Option<(usize, &'static str)> {
    let rest = key.strip_prefix("wheels.")?;
    let (i, field) = rest.split_once('.')?;
    let i: usize = i.parse().ok().filter(|i| *i < 4)?;
    WHEEL_FIELDS.iter().find(|(f, ..)| *f == field).map(|(f, ..)| (i, *f))
}

/// A readable name for a key, for error messages.
fn label_of(key: &str) -> String {
    if let Some((i, field)) = parse_wheel_key(key) {
        let label = WHEEL_FIELDS.iter().find(|(f, ..)| *f == field).map_or(field, |(_, l, _)| *l);
        return format!("{} wheel {}", WHEEL_LONG[i], label.to_lowercase());
    }
    GROUPS.iter().flat_map(|(_, ps)| ps.iter()).chain(WHEEL_SHARED).find(|p| p.name == key).map_or(key.to_string(), |p| p.label.to_string())
}

// --- Top view ---

/// The robot to scale, facing up, with draggable wheels.
struct Drawing {
    robot: RobotConfig,
    ball_radius: f64,
    selected: usize,
}

#[derive(Default)]
struct DragState {
    dragging: Option<usize>,
}

/// How the drawing maps meters to the screen: forward is up.
struct View {
    center: Point,
    scale: f32,
}

impl View {
    fn new(bounds: Rectangle, robot: &RobotConfig, ball_radius: f64) -> Self {
        let up = robot.radius.max(robot.front_distance + 2.0 * ball_radius + robot.kick_reach) + 0.025;
        let down = robot.radius + 0.025;
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

    fn wheel(&self, robot: &RobotConfig, i: usize) -> Point {
        let phi = robot.wheels[i].angle;
        self.at(robot.wheel_distance * phi.cos(), robot.wheel_distance * phi.sin())
    }

    fn wheel_at(&self, robot: &RobotConfig, pos: Point) -> Option<usize> {
        let grab = (0.022 * self.scale).max(12.0);
        (0..4)
            .map(|i| (i, self.wheel(robot, i).distance(pos)))
            .filter(|(_, d)| *d <= grab)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i)
    }
}

impl canvas::Program<EditorMessage> for Drawing {
    type State = DragState;

    fn update(&self, state: &mut DragState, event: &canvas::Event, bounds: Rectangle, cursor: mouse::Cursor) -> Option<canvas::Action<EditorMessage>> {
        let view = View::new(bounds, &self.robot, self.ball_radius);
        match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let wheel = view.wheel_at(&self.robot, cursor.position_in(bounds)?)?;
                state.dragging = Some(wheel);
                Some(canvas::Action::publish(EditorMessage::SelectWheel(wheel)).and_capture())
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let wheel = state.dragging?;
                let pos = cursor.position_in(bounds)?;
                let (dx, dy) = ((pos.x - view.center.x) as f64, (pos.y - view.center.y) as f64);
                let degrees = (-dx).atan2(-dy).to_degrees();
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
        let view = View::new(bounds, &self.robot, self.ball_radius);
        match cursor.position_in(bounds).and_then(|pos| view.wheel_at(&self.robot, pos)) {
            Some(_) => mouse::Interaction::Grab,
            None => mouse::Interaction::default(),
        }
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
                    let (a, b) = if vertical { (Point::new(c, 0.0), Point::new(c, bounds.height)) } else { (Point::new(0.0, c), Point::new(bounds.width, c)) };
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
            b.arc(Arc { center: view.center, radius: r.radius as f32 * s, start_angle: iced::Radians(up + cut), end_angle: iced::Radians(up + std::f32::consts::TAU - cut) });
            b.close();
        });
        frame.fill(&body, Color { a: 0.55, ..theme::SSL_BLUE });
        frame.stroke(&body, Stroke::default().with_color(theme::SSL_BLUE).with_width(1.5));
        frame.stroke(&Path::line(view.at(r.front_distance, half_face), view.at(r.front_distance, -half_face)), Stroke::default().with_color(theme::TEXT).with_width(3.0));

        // Forward axis.
        let dashed = Stroke { line_dash: LineDash { segments: &[4.0, 4.0], offset: 0 }, ..Stroke::default().with_color(Color { a: 0.5, ..theme::TEXT }).with_width(1.0) };
        frame.stroke(&Path::line(view.center, view.at(r.front_distance, 0.0)), dashed);

        // The selected wheel's angle from the forward axis.
        let sel = &r.wheels[self.selected];
        let arc_r = (r.wheel_distance * 0.5) as f32 * s;
        let (a0, a1) = (View::screen_angle(0.0), View::screen_angle(sel.angle));
        // Screen angles grow clockwise: a CCW (positive) robot angle sweeps from it back to forward.
        let (start, end) = if sel.angle >= 0.0 { (a1, a0) } else { (a0, a1) };
        let end = start + (end - start).rem_euclid(std::f32::consts::TAU);
        let arc = Path::new(|b| b.arc(Arc { center: view.center, radius: arc_r, start_angle: iced::Radians(start), end_angle: iced::Radians(end) }));
        frame.stroke(&arc, Stroke::default().with_color(theme::ACCENT_FG).with_width(1.2));
        frame.fill_text(Text {
            content: format!("{:.0}°", sel.angle.to_degrees()),
            position: view.center + View::dir(sel.angle / 2.0) * (arc_r + 14.0),
            color: theme::ACCENT_FG,
            size: iced::Pixels(11.0),
            font: theme::MONO_BOLD,
            align_x: Horizontal::Center.into(),
            align_y: Vertical::Center.into(),
            ..Text::default()
        });

        // Wheels: axle from the center, the wheel across it (it rolls at right
        // angles to its axle). Brightness shows grip; selected or hovered is accent.
        let hovered = cursor.position_in(bounds).and_then(|pos| view.wheel_at(r, pos));
        let wheel_len = (0.028 * s).max(10.0);
        let wheel_w = (0.011 * s).max(5.0);
        let max_grip = r.wheels.iter().map(|w| w.grip).fold(1.0_f64, f64::max);
        for (i, w) in r.wheels.iter().enumerate() {
            let c = view.wheel(r, i);
            let roll = View::dir(w.angle + PI / 2.0);
            let active = state.dragging == Some(i) || (state.dragging.is_none() && hovered == Some(i));
            let selected = i == self.selected;
            frame.stroke(&Path::line(view.center, c), Stroke::default().with_color(Color { a: 0.3, ..theme::TEXT }).with_width(1.0));
            let base = if selected || active { theme::ACCENT } else { theme::TEXT_2 };
            let alpha = (0.35 + 0.65 * (w.grip / max_grip)) as f32;
            let wheel = Path::line(c - roll * (wheel_len / 2.0), c + roll * (wheel_len / 2.0));
            frame.stroke(&wheel, Stroke { line_cap: canvas::LineCap::Round, ..Stroke::default().with_color(Color { a: alpha, ..base }).with_width(wheel_w) });
            for k in [-1.0, 0.0, 1.0] {
                let p = c + roll * (k * wheel_len / 3.2);
                let across = View::dir(w.angle) * (wheel_w / 2.0);
                frame.stroke(&Path::line(p - across, p + across), Stroke::default().with_color(theme::FIELD_FLOOR).with_width(1.0));
            }
            frame.fill_text(Text {
                content: WHEEL_NAMES[i].to_string(),
                position: view.center + View::dir(w.angle) * ((r.radius as f32 * s) + 14.0),
                color: if selected { theme::ACCENT_FG } else { theme::MUTED },
                size: iced::Pixels(10.0),
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
        label(&mut frame, format!("Ø {:.0} mm · face {:.0} mm · {} kg", r.radius * 2000.0, half_face * 2000.0, fmt(r.mass, 2)), 8.0, Horizontal::Left);
        label(&mut frame, if cm >= 4.0 { "grid 1 cm".into() } else { String::new() }, bounds.width - 8.0, Horizontal::Right);

        vec![frame.into_geometry()]
    }
}

// --- Envelope ---

/// Top speed (or acceleration) in every direction, forward up, against the
/// robot-wide limit circle.
struct Envelope {
    robot: RobotConfig,
    plot: Plot,
}

impl Envelope {
    fn limit(&self) -> f64 {
        match self.plot {
            Plot::Speed => self.robot.max_speed,
            Plot::Accel => self.robot.max_accel,
        }
    }

    fn value(&self, phi: f64) -> f64 {
        match self.plot {
            Plot::Speed => self.robot.top_speed_towards(phi),
            Plot::Accel => self.robot.top_accel_towards(phi),
        }
    }
}

impl<M> canvas::Program<M> for Envelope {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let r = &self.robot;
        let center = Point::new(bounds.width / 2.0, bounds.height / 2.0);
        let radius = (bounds.width.min(bounds.height) / 2.0 - 22.0).max(10.0);
        let max = self.limit().max(1e-6);
        let to = |phi: f64, v: f64| center + View::dir(phi) * ((v / max) as f32 * radius);

        // Rings every unit and spokes every 45°.
        let faint = Stroke::default().with_color(Color { a: 0.07, ..theme::FIELD_LINE }).with_width(1.0);
        let step = if max > 12.0 { 5.0 } else { 1.0 };
        let mut v = step;
        while v < max {
            frame.stroke(&Path::circle(center, (v / max) as f32 * radius), faint);
            v += step;
        }
        for k in 0..8 {
            frame.stroke(&Path::line(center, to(k as f64 * PI / 4.0, max)), faint);
        }
        frame.stroke(&Path::circle(center, radius), Stroke { line_dash: LineDash { segments: &[4.0, 4.0], offset: 0 }, ..Stroke::default().with_color(theme::MUTED).with_width(1.0) });

        let color = match self.plot {
            Plot::Speed => theme::ACCENT,
            Plot::Accel => theme::GREEN,
        };
        let shape = Path::new(|b| {
            for k in 0..=360 {
                let phi = (k as f64).to_radians();
                let p = to(phi, self.value(phi).min(max));
                if k == 0 { b.move_to(p) } else { b.line_to(p) }
            }
            b.close();
        });
        frame.fill(&shape, Color { a: 0.22, ..color });
        frame.stroke(&shape, Stroke::default().with_color(color).with_width(1.8));

        // Each wheel's rolling directions as ticks on the rim.
        for w in &r.wheels {
            for side in [PI / 2.0, -PI / 2.0] {
                let a = w.angle + side;
                frame.stroke(&Path::line(to(a, max * 1.02), to(a, max * 1.1)), Stroke::default().with_color(theme::TEXT_2).with_width(2.0));
            }
        }

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
        let unit = match self.plot {
            Plot::Speed => "m/s",
            Plot::Accel => "m/s²",
        };
        label(&mut frame, "forward".into(), Point::new(center.x, center.y - radius - 12.0));
        label(&mut frame, format!("limit {} {unit}", fmt(max, 3)), Point::new(center.x, center.y + radius + 12.0));

        vec![frame.into_geometry()]
    }
}

fn envelope_stats<'a>(r: &RobotConfig, plot: Plot) -> Element<'a, EditorMessage> {
    let f = |phi: f64| match plot {
        Plot::Speed => r.top_speed_towards(phi),
        Plot::Accel => r.top_accel_towards(phi),
    };
    let (lowest, at) = (0..360).map(|k| (f((k as f64).to_radians()), k)).fold((f64::MAX, 0), |m, x| if x.0 < m.0 { x } else { m });
    let stat = |label: &'static str, value: String| {
        column![text(label).size(10).color(theme::MUTED), text(value).size(13).font(theme::MONO_BOLD).color(theme::TEXT)]
            .spacing(1)
            .width(Length::Fill)
            .align_x(Horizontal::Center)
    };
    let fourth = match plot {
        Plot::Speed => {
            let by_wheels = r.wheels.iter().filter(|w| w.max_speed > 0.0).map(|w| w.max_speed / r.wheel_distance).fold(f64::INFINITY, f64::min);
            stat("turn rad/s", fmt(r.max_omega.min(by_wheels), 3))
        }
        Plot::Accel => stat("backward", fmt(f(PI), 3)),
    };
    let mut col = column![row![
        stat("forward", fmt(f(0.0), 3)),
        stat("sideways", fmt(f(PI / 2.0).min(f(-PI / 2.0)), 3)),
        stat("lowest", format!("{} @{at}°", fmt(lowest, 3))),
        fourth,
    ]]
    .spacing(4);
    let limited = match plot {
        Plot::Speed => r.wheels.iter().any(|w| w.max_speed > 0.0),
        Plot::Accel => r.wheels.iter().any(|w| w.torque > 0.0) || r.grip_accel > 0.0,
    };
    if !limited {
        col = col.push(
            text(match plot {
                Plot::Speed => "No wheel has a top speed: the robot reaches max speed every way. Give the wheels one to see what their angles do.",
                Plot::Accel => "No wheel has a torque limit: the robot reaches max acceleration every way. Set torques to see what the wheels allow.",
            })
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
    let s = if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.').to_string() } else { s };
    if s == "-0" { "0".into() } else { s }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sender::simulator::{console, SimLink};
    use crate::world::World;

    #[test]
    fn every_robot_parameter_has_a_field() {
        let robot = RobotConfig::default();
        for k in keys() {
            assert!(get(&robot, &k).is_some(), "{k}");
        }
        // Every number in the profile: the flat ones plus 4 per wheel.
        let json = serde_json::to_value(&robot).unwrap();
        let flat = json.as_object().unwrap().values().filter(|v| v.is_number()).count();
        assert_eq!(keys().len(), flat + 4 * WHEEL_FIELDS.len());
    }

    #[test]
    fn fields_round_trip_with_angles_in_degrees() {
        let panel = RobotEditorPanel::default();
        assert_eq!(panel.typed["wheels.0.angle"], "60");
        assert_eq!(panel.typed["wheels.2.angle"], "-135");
        assert_eq!(panel.draft().unwrap(), RobotConfig::default());
    }

    #[test]
    fn mirrored_wheels_move_and_change_together() {
        let mut panel = RobotEditorPanel::default();
        panel.update(EditorMessage::DragWheel(0, 45.0));
        assert_eq!((panel.typed["wheels.0.angle"].as_str(), panel.typed["wheels.3.angle"].as_str()), ("45", "-45"));
        panel.update(EditorMessage::Edit("wheels.1.torque".into(), "0.06".into()));
        assert_eq!(panel.typed["wheels.2.torque"], "0.06");

        panel.update(EditorMessage::Mirror(false));
        panel.update(EditorMessage::DragWheel(3, -80.0));
        assert_eq!((panel.typed["wheels.0.angle"].as_str(), panel.typed["wheels.3.angle"].as_str()), ("45", "-80"));
        assert!(panel.draft().is_ok());
    }

    #[test]
    fn apply_is_one_checked_command_and_is_confirmed() {
        let mut link = SimLink::new(SimConfig::default());
        let mut world = World::new(0, 0, 9.0, 6.0);
        let mut panel = RobotEditorPanel::default();
        panel.set_config(Some(link.config().clone()));
        assert!(panel.changed().is_empty());

        // Shrinking the body and its face together: fine as a whole, though
        // the radius alone would be rejected.
        panel.update(EditorMessage::Edit("radius".into(), "0.07".into()));
        panel.update(EditorMessage::Edit("front_distance".into(), "0.06".into()));
        panel.update(EditorMessage::Edit("wheel_distance".into(), "0.06".into()));
        panel.update(EditorMessage::Edit("wheels.0.grip".into(), "0.5".into()));
        panel.update(EditorMessage::DragWheel(1, 120.0));
        let commands = panel.update(EditorMessage::Apply);
        assert_eq!(commands.len(), 1, "{commands:?}");
        assert!(commands[0].starts_with("sim robot set "));

        let reply = console::execute(&commands[0], Some(&mut link), &mut world);
        assert!(!reply.iter().any(|r| r.starts_with("error")), "{reply:?}");
        panel.set_config(Some(link.config().clone()));
        assert_eq!(link.config().robot, panel.draft().unwrap());
        assert!(panel.changed().is_empty(), "{:?}", panel.changed());
        assert!(matches!(&panel.status, Some(Ok(m)) if m.contains("now runs")));
    }

    #[test]
    fn invalid_fields_block_apply() {
        let mut panel = RobotEditorPanel::default();
        panel.set_config(Some(SimConfig::default()));
        panel.update(EditorMessage::Edit("slide_grip".into(), "1.5".into()));
        assert!(panel.update(EditorMessage::Apply).is_empty());
        assert!(matches!(&panel.status, Some(Err(e)) if e.contains("slide_grip")));
    }
}
