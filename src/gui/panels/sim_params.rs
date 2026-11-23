// gui/panels/sim_params.rs — Robot and ball parameters of the built-in simulator
//
// Shows the parameters the simulator runs with (sent by the engine whenever
// they may have changed) and edits them through the `sim robot|ball <name>
// <value>` console commands, so the console reports every change.
//
// Each parameter has a slider (applied when released) and a number field
// (applied with Enter or Apply). "Save configuration" applies what is pending
// and writes the running values to simulator.toml (`sim save`).

use iced::widget::{button, column, container, row, slider, text, text_input, Space};
use iced::{alignment, Alignment, Background, Border, Color, Element, Length, Theme};
use ssl_sim::SimConfig;
use std::collections::BTreeMap;

use crate::gui::theme::{self, btn, Btn};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Group {
    Robot,
    Ball,
    Vision,
    Radio,
}

impl Group {
    fn word(self) -> &'static str {
        match self {
            Group::Robot => "robot",
            Group::Ball => "ball",
            Group::Vision => "vision",
            Group::Radio => "radio",
        }
    }

    fn params(self) -> &'static [Param] {
        match self {
            Group::Robot => ROBOT,
            Group::Ball => BALL,
            Group::Vision => VISION,
            Group::Radio => RADIO,
        }
    }
}

/// One editable parameter: name in the config, label, unit, and the slider's
/// min, max and step. The slider widens to reach a value outside its range;
/// the number field takes any value.
struct Param(&'static str, &'static str, &'static str, f64, f64, f64);

const ROBOT: &[Param] = &[
    Param("max_speed", "Max speed", "m/s", 0.0, 6.0, 0.05),
    Param("max_accel", "Max acceleration", "m/s²", 0.0, 12.0, 0.1),
    Param("max_omega", "Max angular speed", "rad/s", 0.0, 30.0, 0.5),
    Param("max_alpha", "Max angular accel.", "rad/s²", 0.0, 120.0, 1.0),
    Param("grip_accel", "Grip (0 = no slip)", "m/s²", 0.0, 20.0, 0.1),
    Param("slide_grip", "Grip while sliding", "0–1", 0.0, 1.0, 0.01),
    Param("slip_spin", "Spin while sliding", "rad/m", 0.0, 20.0, 0.1),
    Param("mass", "Mass", "kg", 0.5, 5.0, 0.05),
    Param("wheel_radius", "Wheel radius", "m", 0.01, 0.05, 0.0005),
    Param("wheel_distance", "Wheel distance", "m", 0.03, 0.09, 0.001),
    Param("kick_x_speed", "Flat kick speed", "m/s", 0.0, 10.0, 0.1),
    Param("max_kick_speed", "Max kick speed", "m/s", 0.0, 10.0, 0.1),
    Param("kick_reach", "Kick reach", "m", 0.0, 0.1, 0.001),
    Param("radius", "Radius", "m", 0.05, 0.1, 0.001),
    Param("front_distance", "Front distance", "m", 0.03, 0.09, 0.001),
    Param("body_restitution", "Body bounce", "0–1", 0.0, 1.0, 0.01),
    Param("front_restitution", "Front bounce", "0–1", 0.0, 1.0, 0.01),
];

const BALL: &[Param] = &[
    Param("slide_decel", "Slide deceleration", "m/s²", 0.0, 10.0, 0.05),
    Param("roll_decel", "Roll deceleration", "m/s²", 0.0, 2.0, 0.01),
    Param("wall_restitution", "Wall bounce", "0–1", 0.0, 1.0, 0.01),
    Param("radius", "Radius", "m", 0.01, 0.04, 0.0005),
];

const VISION: &[Param] = &[
    Param("position_noise", "Position noise (σ)", "m", 0.0, 0.02, 0.0005),
    Param("angle_noise", "Heading noise (σ)", "rad", 0.0, 0.1, 0.001),
    Param("delay", "Vision delay", "s", 0.0, 0.2, 0.001),
];

const RADIO: &[Param] = &[Param("delay", "Command delay", "s", 0.0, 0.2, 0.001)];

#[derive(Debug, Clone)]
pub enum SimParamsMessage {
    Edit(Group, &'static str, String),
    /// A slider moved: the value becomes an edit, applied when it is released.
    Slide(Group, &'static str, f64),
    /// Sends every edited value to the simulator.
    Apply,
    /// Drops the edits.
    Revert,
    /// Applies the edits, then writes the running parameters to simulator.toml (`sim save`).
    Save,
}

#[derive(Default)]
pub struct SimParamsPanel {
    /// What the simulator runs with; None while it is off.
    config: Option<SimConfig>,
    /// Typed values not applied yet.
    edits: BTreeMap<(Group, &'static str), String>,
}

impl SimParamsPanel {
    /// The engine sent the simulator's parameters: edits it now runs with are done.
    pub fn set_config(&mut self, config: Option<SimConfig>) {
        if let Some(config) = &config {
            self.edits.retain(|(group, name), typed| typed.trim().parse::<f64>().ok() != value(config, *group, name));
        } else {
            self.edits.clear();
        }
        self.config = config;
    }

    /// Handles a message; returns the console commands to run.
    pub fn update(&mut self, message: SimParamsMessage) -> Vec<String> {
        match message {
            SimParamsMessage::Slide(group, name, v) => {
                let step = param(group, name).map_or(0.001, |p| p.5);
                self.update(SimParamsMessage::Edit(group, name, format_value(snap(v, step))))
            }
            SimParamsMessage::Edit(group, name, typed) => {
                let unchanged = self.config.as_ref().is_some_and(|c| typed.trim().parse::<f64>().ok() == value(c, group, name));
                if unchanged {
                    self.edits.remove(&(group, name));
                } else {
                    self.edits.insert((group, name), typed);
                }
                Vec::new()
            }
            SimParamsMessage::Apply => self.apply_commands(),
            SimParamsMessage::Revert => {
                self.edits.clear();
                Vec::new()
            }
            SimParamsMessage::Save => {
                if self.config.is_none() || self.error().is_some() {
                    return Vec::new();
                }
                let mut commands = self.apply_commands();
                commands.push("sim save".to_string());
                commands
            }
        }
    }

    /// The console commands that apply the edits; none while any is invalid.
    fn apply_commands(&self) -> Vec<String> {
        if self.error().is_some() {
            return Vec::new();
        }
        self.edits
            .iter()
            .map(|((group, name), typed)| format!("sim {} {name} {}", group.word(), typed.trim()))
            .collect()
    }

    /// Why the edits cannot be applied, if they cannot.
    fn error(&self) -> Option<String> {
        let config = self.config.as_ref()?;
        let mut json = serde_json::to_value(config).ok()?;
        for ((group, name), typed) in &self.edits {
            let label = format!("{}.{name}", group.word());
            let Ok(v) = typed.trim().parse::<f64>() else {
                return Some(format!("{label} is not a number"));
            };
            if !v.is_finite() || v < 0.0 {
                return Some(format!("{label} must be a number >= 0"));
            }
            json[group.word()][*name] = serde_json::json!(v);
        }
        let candidate: SimConfig = serde_json::from_value(json).ok()?;
        candidate.validate().err()
    }

    pub fn view(&self) -> Element<'_, SimParamsMessage> {
        let Some(config) = &self.config else {
            return container(
                column![
                    text("Simulator is off").size(15).font(theme::SANS_BOLD),
                    text("Turn on SIM in the toolbar to see and change its robot and ball parameters.")
                        .size(12)
                        .color(theme::MUTED)
                        .align_x(alignment::Horizontal::Center),
                ]
                .spacing(6)
                .align_x(Alignment::Center),
            )
            .padding(24)
            .center(Length::Fill)
            .into();
        };

        let error = self.error();
        let pending = self.edits.len();
        let can_apply = pending > 0 && error.is_none();

        let status: Element<'_, SimParamsMessage> = match (&error, pending) {
            (Some(e), _) => text(e.clone()).size(11).color(theme::RED_FG).into(),
            (None, 0) => text("Running values").size(11).color(theme::MUTED).into(),
            (None, n) => text(format!("{n} change{} not applied", if n == 1 { "" } else { "s" }))
                .size(11)
                .color(theme::ACCENT_FG)
                .into(),
        };

        let action = |label: &'static str, kind: Btn, message: Option<SimParamsMessage>| {
            button(text(label).size(12))
                .padding([4, 10])
                .style(btn(kind))
                .on_press_maybe(message)
        };

        let footer = row![
            status,
            Space::new().width(Length::Fill),
            action("Revert", Btn::Ghost, (pending > 0).then_some(SimParamsMessage::Revert)),
            action("Apply", Btn::Toggle(can_apply), can_apply.then_some(SimParamsMessage::Apply)),
            action("Save configuration", Btn::Play, error.is_none().then_some(SimParamsMessage::Save)),
        ]
        .spacing(6)
        .align_y(Alignment::Center);

        // Two columns keep the window short: the robot on the left, the rest on the right.
        let sections = row![
            column![self.section(config, Group::Robot)].width(Length::Fill),
            column![
                self.section(config, Group::Ball),
                self.section(config, Group::Vision),
                self.section(config, Group::Radio),
            ]
            .spacing(8)
            .width(Length::Fill),
        ]
        .spacing(8);

        column![
            sections,
            container(footer).padding([4, 2]),
            text("Sliders apply when released. Save configuration applies pending changes and writes them to simulator.toml, read every time SIM turns on.")
                .size(10)
                .color(theme::MUTED),
        ]
        .spacing(6)
        .padding(10)
        .into()
    }

    fn section<'a>(&'a self, config: &SimConfig, group: Group) -> Element<'a, SimParamsMessage> {
        let title = match group {
            Group::Robot => "ROBOT",
            Group::Ball => "BALL",
            Group::Vision => "VISION · WHAT THE AI SEES",
            Group::Radio => "RADIO",
        };
        let mut rows = column![text(title).size(11).font(theme::MONO_BOLD).color(theme::MUTED)].spacing(3);
        for &Param(name, label, unit, min, max, step) in group.params() {
            let edited = self.edits.get(&(group, name));
            let running = value(config, group, name).unwrap_or(min);
            let current = edited
                .and_then(|typed| typed.trim().parse::<f64>().ok())
                .filter(|v| v.is_finite())
                .unwrap_or(running);
            let range = min.min(current).min(running)..=max.max(current).max(running);
            let bar = slider(range, current, move |v| SimParamsMessage::Slide(group, name, v))
                .step(step)
                .on_release(SimParamsMessage::Apply)
                .width(Length::Fill);
            let shown = match edited {
                Some(typed) => typed.clone(),
                None => value(config, group, name).map(format_value).unwrap_or_default(),
            };
            let state = match edited {
                None => Field::Clean,
                Some(typed) if typed.trim().parse::<f64>().is_ok_and(|v| v.is_finite() && v >= 0.0) => Field::Edited,
                Some(_) => Field::Invalid,
            };
            let input = text_input("", &shown)
                .on_input(move |typed| SimParamsMessage::Edit(group, name, typed))
                .on_submit(SimParamsMessage::Apply)
                .font(theme::MONO)
                .size(12)
                .padding([2, 6])
                .align_x(alignment::Horizontal::Right)
                .width(Length::Fixed(72.0))
                .style(move |t, s| field_style(t, s, state));
            rows = rows.push(
                row![
                    text(label)
                        .size(12)
                        .color(if edited.is_some() { theme::TEXT } else { theme::TEXT_2 })
                        .width(Length::Fixed(128.0)),
                    bar,
                    input,
                    text(unit).size(11).font(theme::MONO).color(theme::MUTED).width(Length::Fixed(40.0)),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            );
        }
        container(rows)
            .padding([8, 10])
            .width(Length::Fill)
            .style(|_| container::Style {
                background: Some(Background::Color(theme::GROUP)),
                border: Border { color: theme::DIVIDER, width: 1.0, radius: 6.0.into() },
                text_color: Some(theme::TEXT),
                ..Default::default()
            })
            .into()
    }
}

#[derive(Debug, Clone, Copy)]
enum Field {
    Clean,
    Edited,
    Invalid,
}

fn field_style(t: &Theme, status: text_input::Status, field: Field) -> text_input::Style {
    let mut style = theme::input(t, status);
    let accent: Option<Color> = match field {
        Field::Clean => None,
        Field::Edited => Some(theme::ACCENT),
        Field::Invalid => Some(theme::RED),
    };
    if let Some(c) = accent {
        style.border.color = c;
        style.value = if matches!(field, Field::Invalid) { theme::RED_FG } else { theme::ACCENT_FG };
    }
    style
}

/// A parameter's value in `config`, looked up by name.
fn value(config: &SimConfig, group: Group, name: &str) -> Option<f64> {
    serde_json::to_value(config).ok()?.get(group.word())?.get(name)?.as_f64()
}

fn param(group: Group, name: &str) -> Option<&'static Param> {
    group.params().iter().find(|p| p.0 == name)
}

/// `v` rounded to the decimals of `step`, so slider values read cleanly (0.45, not 0.45000000000000007).
fn snap(v: f64, step: f64) -> f64 {
    let decimals = (-step.log10()).ceil().clamp(0.0, 9.0) as usize;
    format!("{v:.decimals$}").parse().unwrap_or(v)
}

/// Shortest text that reads back as the same number.
fn format_value(v: f64) -> String {
    let s = format!("{v}");
    if s.contains('.') || s.contains('e') { s } else { format!("{s}.0") }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn panel() -> SimParamsPanel {
        let mut panel = SimParamsPanel::default();
        panel.set_config(Some(SimConfig::default()));
        panel
    }

    #[test]
    fn every_parameter_exists_in_the_config() {
        let config = SimConfig::default();
        for group in [Group::Robot, Group::Ball, Group::Vision, Group::Radio] {
            for &Param(name, _, _, min, max, step) in group.params() {
                let v = value(&config, group, name);
                assert!(v.is_some(), "{} {name}", group.word());
                assert!(min < max && step > 0.0, "{} {name} slider range", group.word());
                assert!((min..=max).contains(&v.unwrap()), "{} {name} default outside the slider", group.word());
            }
        }
    }

    #[test]
    fn applies_edits_as_console_commands() {
        let mut panel = panel();
        panel.update(SimParamsMessage::Edit(Group::Robot, "max_speed", "2.5".into()));
        panel.update(SimParamsMessage::Edit(Group::Ball, "roll_decel", " 0.4 ".into()));
        assert_eq!(panel.update(SimParamsMessage::Apply), ["sim robot max_speed 2.5", "sim ball roll_decel 0.4"]);

        // Once the simulator runs with them, they are no longer edits.
        let mut config = SimConfig::default();
        config.robot.max_speed = 2.5;
        config.ball.roll_decel = 0.4;
        panel.set_config(Some(config));
        assert!(panel.edits.is_empty());
    }

    #[test]
    fn does_not_apply_invalid_edits() {
        let mut panel = panel();
        panel.update(SimParamsMessage::Edit(Group::Robot, "max_speed", "fast".into()));
        assert!(panel.update(SimParamsMessage::Apply).is_empty());
        assert_eq!(panel.error().as_deref(), Some("robot.max_speed is not a number"));

        panel.update(SimParamsMessage::Edit(Group::Robot, "max_speed", "3".into()));
        panel.update(SimParamsMessage::Edit(Group::Ball, "wall_restitution", "1.5".into()));
        assert!(panel.update(SimParamsMessage::Apply).is_empty());
        assert!(panel.error().unwrap().contains("wall_restitution"));
    }

    #[test]
    fn slider_edits_are_rounded_to_the_step() {
        let mut panel = panel();
        panel.update(SimParamsMessage::Slide(Group::Ball, "roll_decel", 0.35 + 0.1));
        assert_eq!(panel.update(SimParamsMessage::Apply), ["sim ball roll_decel 0.45"]);
    }

    #[test]
    fn save_applies_pending_edits_first() {
        let mut panel = panel();
        assert_eq!(panel.update(SimParamsMessage::Save), ["sim save"]);
        panel.update(SimParamsMessage::Edit(Group::Robot, "max_speed", "2.5".into()));
        assert_eq!(panel.update(SimParamsMessage::Save), ["sim robot max_speed 2.5", "sim save"]);
        panel.update(SimParamsMessage::Edit(Group::Robot, "max_speed", "fast".into()));
        assert!(panel.update(SimParamsMessage::Save).is_empty());
    }

    #[test]
    fn typing_the_running_value_is_not_an_edit() {
        let mut panel = panel();
        let speed = SimConfig::default().robot.max_speed;
        panel.update(SimParamsMessage::Edit(Group::Robot, "max_speed", format!("{speed}")));
        assert!(panel.edits.is_empty());
    }
}
