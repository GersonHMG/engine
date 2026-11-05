// gui/panels/sim_params.rs — Robot and ball parameters of the built-in simulator
//
// Shows the parameters the simulator runs with (sent by the engine whenever
// they may have changed) and edits them through the `sim robot|ball <name>
// <value>` console commands, so the console reports every change.

use iced::widget::{button, column, container, row, text, text_input, Space};
use iced::{alignment, Alignment, Background, Border, Color, Element, Length, Theme};
use ssl_sim::SimConfig;
use std::collections::BTreeMap;

use crate::gui::theme::{self, btn, Btn};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Group {
    Robot,
    Ball,
}

impl Group {
    fn word(self) -> &'static str {
        match self {
            Group::Robot => "robot",
            Group::Ball => "ball",
        }
    }

    fn params(self) -> &'static [Param] {
        match self {
            Group::Robot => ROBOT,
            Group::Ball => BALL,
        }
    }
}

/// One editable parameter: name in the config, label, unit.
struct Param(&'static str, &'static str, &'static str);

const ROBOT: &[Param] = &[
    Param("max_speed", "Max speed", "m/s"),
    Param("max_accel", "Max acceleration", "m/s²"),
    Param("max_omega", "Max angular speed", "rad/s"),
    Param("max_alpha", "Max angular accel.", "rad/s²"),
    Param("kick_x_speed", "Flat kick speed", "m/s"),
    Param("max_kick_speed", "Max kick speed", "m/s"),
    Param("kick_reach", "Kick reach", "m"),
    Param("radius", "Radius", "m"),
    Param("front_distance", "Front distance", "m"),
    Param("body_restitution", "Body bounce", "0–1"),
    Param("front_restitution", "Front bounce", "0–1"),
];

const BALL: &[Param] = &[
    Param("slide_decel", "Slide deceleration", "m/s²"),
    Param("roll_decel", "Roll deceleration", "m/s²"),
    Param("wall_restitution", "Wall bounce", "0–1"),
    Param("radius", "Radius", "m"),
];

#[derive(Debug, Clone)]
pub enum SimParamsMessage {
    Edit(Group, &'static str, String),
    /// Sends every edited value to the simulator.
    Apply,
    /// Drops the edits.
    Revert,
    /// Writes the running parameters to simulator.toml (`sim save`).
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
            SimParamsMessage::Edit(group, name, typed) => {
                let unchanged = self.config.as_ref().is_some_and(|c| typed.trim().parse::<f64>().ok() == value(c, group, name));
                if unchanged {
                    self.edits.remove(&(group, name));
                } else {
                    self.edits.insert((group, name), typed);
                }
                Vec::new()
            }
            SimParamsMessage::Apply => {
                if self.error().is_some() {
                    return Vec::new();
                }
                self.edits
                    .iter()
                    .map(|((group, name), typed)| format!("sim {} {name} {}", group.word(), typed.trim()))
                    .collect()
            }
            SimParamsMessage::Revert => {
                self.edits.clear();
                Vec::new()
            }
            SimParamsMessage::Save => vec!["sim save".to_string()],
        }
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
            action("Save", Btn::Neutral, (pending == 0).then_some(SimParamsMessage::Save)),
        ]
        .spacing(6)
        .align_y(Alignment::Center);

        column![
            self.section(config, Group::Robot),
            self.section(config, Group::Ball),
            container(footer).padding([6, 2]),
            text("Save writes the running values to simulator.toml, read every time SIM turns on.")
                .size(10)
                .color(theme::MUTED),
        ]
        .spacing(10)
        .padding(10)
        .into()
    }

    fn section<'a>(&'a self, config: &SimConfig, group: Group) -> Element<'a, SimParamsMessage> {
        let title = match group {
            Group::Robot => "ROBOT",
            Group::Ball => "BALL",
        };
        let mut rows = column![text(title).size(11).font(theme::MONO_BOLD).color(theme::MUTED)].spacing(4);
        for &Param(name, label, unit) in group.params() {
            let edited = self.edits.get(&(group, name));
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
                .padding([3, 6])
                .align_x(alignment::Horizontal::Right)
                .width(Length::Fixed(84.0))
                .style(move |t, s| field_style(t, s, state));
            rows = rows.push(
                row![
                    text(label).size(12).color(if edited.is_some() { theme::TEXT } else { theme::TEXT_2 }).width(Length::Fill),
                    input,
                    text(unit).size(11).font(theme::MONO).color(theme::MUTED).width(Length::Fixed(44.0)),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
        }
        container(rows)
            .padding([10, 12])
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
        for group in [Group::Robot, Group::Ball] {
            for Param(name, ..) in group.params() {
                assert!(value(&config, group, name).is_some(), "{} {name}", group.word());
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
    fn typing_the_running_value_is_not_an_edit() {
        let mut panel = panel();
        let speed = SimConfig::default().robot.max_speed;
        panel.update(SimParamsMessage::Edit(Group::Robot, "max_speed", format!("{speed}")));
        assert!(panel.edits.is_empty());
    }
}
