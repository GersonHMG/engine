// gui/panels/robot_settings.rs — Bang-bang motion limits, for all robots and per robot
//
// Shows the limits move_to/move_direct plan with (motion::settings) and edits
// them through the `motion ...` console commands, so the console reports
// every change. A robot's field left empty uses the all-robots value, shown
// greyed as the placeholder.

use iced::widget::{button, column, container, row, text, text_input, Space};
use iced::{alignment, Alignment, Background, Border, Color, Element, Length, Theme};
use std::collections::BTreeMap;

use crate::gui::theme::{self, btn, Btn};
use crate::motion::settings::{self, Param, RobotSettings, TeamColor};

/// Whose value: every robot's, or one robot's own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Target {
    All,
    Robot(TeamColor, i32),
}

#[derive(Debug, Clone)]
pub enum RobotSettingsMessage {
    Edit(Target, Param, String),
    /// Sends every edited value to the engine.
    Apply,
    /// Drops the edits.
    Revert,
    /// A robot back to the all-robots values.
    Clear(TeamColor, i32),
    /// Writes the limits to robot_settings.toml (`motion save`).
    Save,
}

#[derive(Default)]
pub struct RobotSettingsPanel {
    /// What the engine runs with.
    settings: RobotSettings,
    /// `settings::version()` of `settings`; 0 = not read yet.
    version: u64,
    /// Typed values not applied yet.
    edits: BTreeMap<(Target, Param), String>,
    /// Robots on the field now.
    visible: Vec<(TeamColor, i32)>,
}

impl RobotSettingsPanel {
    /// Reads the engine's limits again if they changed.
    pub fn refresh(&mut self) {
        let version = settings::version();
        if version == self.version {
            return;
        }
        self.version = version;
        self.settings = settings::current();
        // Edits the engine now runs with are done.
        let settings = &self.settings;
        self.edits.retain(|(target, param), typed| parse(*target, *param, typed) != Ok(current(settings, *target, *param)));
    }

    /// The robots on the field (engine ids: team 0 = blue).
    pub fn set_visible(&mut self, robots: impl IntoIterator<Item = (i32, i32)>) {
        self.visible = robots
            .into_iter()
            .filter_map(|(team, id)| TeamColor::from_id(team).map(|team| (team, id)))
            .collect();
    }

    /// Handles a message; returns the console commands to run.
    pub fn update(&mut self, message: RobotSettingsMessage) -> Vec<String> {
        match message {
            RobotSettingsMessage::Edit(target, param, typed) => {
                if parse(target, param, &typed) == Ok(current(&self.settings, target, param)) {
                    self.edits.remove(&(target, param));
                } else {
                    self.edits.insert((target, param), typed);
                }
                Vec::new()
            }
            RobotSettingsMessage::Apply => {
                if self.error().is_some() {
                    return Vec::new();
                }
                self.edits
                    .iter()
                    .map(|((target, param), typed)| {
                        let value = match parse(*target, *param, typed) {
                            Ok(Some(v)) => v.to_string(),
                            _ => "default".to_string(),
                        };
                        match target {
                            Target::All => format!("motion all {} {value}", param.word()),
                            Target::Robot(team, id) => format!("motion {} {id} {} {value}", team.word(), param.word()),
                        }
                    })
                    .collect()
            }
            RobotSettingsMessage::Revert => {
                self.edits.clear();
                Vec::new()
            }
            RobotSettingsMessage::Clear(team, id) => {
                self.edits.retain(|(target, _), _| *target != Target::Robot(team, id));
                vec![format!("motion {} {id} clear", team.word())]
            }
            RobotSettingsMessage::Save => vec!["motion save".to_string()],
        }
    }

    /// Why the edits cannot be applied, if they cannot.
    fn error(&self) -> Option<String> {
        self.edits.iter().find_map(|((target, param), typed)| {
            parse(*target, *param, typed).err().map(|e| match target {
                Target::All => format!("all robots {}: {e}", param.word()),
                Target::Robot(team, id) => format!("{} {id} {}: {e}", team.word(), param.word()),
            })
        })
    }

    /// Robots shown in the per-robot list: on the field, or with their own values.
    fn robots(&self) -> Vec<(TeamColor, i32)> {
        let mut robots: Vec<(TeamColor, i32)> = self.visible.clone();
        robots.extend(self.settings.bangbang.robots.iter().map(|r| (r.team, r.id)));
        robots.sort();
        robots.dedup();
        robots
    }

    pub fn view(&self) -> Element<'_, RobotSettingsMessage> {
        let error = self.error();
        let pending = self.edits.len();
        let can_apply = pending > 0 && error.is_none();

        // All robots
        let mut all = column![section_title("BANG-BANG · ALL ROBOTS")].spacing(4);
        for param in Param::ALL {
            all = all.push(
                row![
                    text(label(param)).size(12).color(theme::TEXT_2).width(Length::Fill),
                    self.input(Target::All, param),
                    text(unit(param)).size(11).font(theme::MONO).color(theme::MUTED).width(Length::Fixed(40.0)),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            );
        }

        // Per robot
        let header = row![
            text("Robot").size(11).color(theme::MUTED).width(Length::Fill),
            text("Accel m/s²").size(11).color(theme::MUTED).width(Length::Fixed(INPUT_W)),
            text("Speed m/s").size(11).color(theme::MUTED).width(Length::Fixed(INPUT_W)),
            text("Ramp m/s²").size(11).color(theme::MUTED).width(Length::Fixed(INPUT_W)),
            Space::new().width(Length::Fixed(RESET_W)),
        ]
        .spacing(6);
        let mut per_robot = column![
            section_title("BANG-BANG · PER ROBOT"),
            text("Empty uses the all-robots value.").size(10).color(theme::MUTED),
            header,
        ]
        .spacing(4);
        let robots = self.robots();
        if robots.is_empty() {
            per_robot = per_robot.push(text("No robots on the field").size(12).color(theme::MUTED));
        }
        for (team, id) in robots {
            let own = self.settings.bangbang.robot(team, id).is_some();
            let edited = self.edits.keys().any(|(t, _)| *t == Target::Robot(team, id));
            let badge_color = match team {
                TeamColor::Blue => theme::SSL_BLUE,
                TeamColor::Yellow => theme::SSL_YELLOW,
            };
            let name = row![
                container(Space::new().width(8).height(8)).style(move |_| container::Style {
                    background: Some(Background::Color(badge_color)),
                    border: Border { radius: 4.0.into(), ..Default::default() },
                    ..Default::default()
                }),
                text(format!("{} {id}", team.word())).size(12).color(if own || edited { theme::TEXT } else { theme::TEXT_2 }),
            ]
            .spacing(6)
            .align_y(Alignment::Center)
            .width(Length::Fill);
            let reset = button(text("Reset").size(11))
                .padding([2, 6])
                .width(Length::Fixed(RESET_W))
                .style(btn(Btn::Ghost))
                .on_press_maybe(own.then_some(RobotSettingsMessage::Clear(team, id)));
            per_robot = per_robot.push(
                row![
                    name,
                    self.input(Target::Robot(team, id), Param::MaxAccel),
                    self.input(Target::Robot(team, id), Param::MaxSpeed),
                    self.input(Target::Robot(team, id), Param::RampAccel),
                    reset,
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            );
        }

        let status: Element<'_, RobotSettingsMessage> = match (&error, pending) {
            (Some(e), _) => text(e.clone()).size(11).color(theme::RED_FG).into(),
            (None, 0) => text("Running values").size(11).color(theme::MUTED).into(),
            (None, n) => text(format!("{n} change{} not applied", if n == 1 { "" } else { "s" }))
                .size(11)
                .color(theme::ACCENT_FG)
                .into(),
        };
        let action = |label: &'static str, kind: Btn, message: Option<RobotSettingsMessage>| {
            button(text(label).size(12)).padding([4, 10]).style(btn(kind)).on_press_maybe(message)
        };
        let footer = row![
            status,
            Space::new().width(Length::Fill),
            action("Revert", Btn::Ghost, (pending > 0).then_some(RobotSettingsMessage::Revert)),
            action("Apply", Btn::Toggle(can_apply), can_apply.then_some(RobotSettingsMessage::Apply)),
            action("Save", Btn::Neutral, (pending == 0).then_some(RobotSettingsMessage::Save)),
        ]
        .spacing(6)
        .align_y(Alignment::Center);

        column![
            group(all.into()),
            group(per_robot.into()),
            container(footer).padding([4, 2]),
            text("Applies at once. Max acceleration and speed plan move_to/move_direct. Command ramp: most any velocity command (also manual control) may change per second, so the wheels do not slip; set it to what the robot can do without sliding, a little above max acceleration (0 = no ramp). Save writes robot_settings.toml, read when the engine starts.")
                .size(10)
                .color(theme::MUTED),
        ]
        .spacing(8)
        .padding(10)
        .into()
    }

    fn input(&self, target: Target, param: Param) -> Element<'_, RobotSettingsMessage> {
        let edited = self.edits.get(&(target, param));
        let shown = match edited {
            Some(typed) => typed.clone(),
            None => current(&self.settings, target, param).map(format_value).unwrap_or_default(),
        };
        let state = match edited {
            None => Field::Clean,
            Some(typed) if parse(target, param, typed).is_ok() => Field::Edited,
            Some(_) => Field::Invalid,
        };
        // A robot's empty field shows the all-robots value it uses.
        let placeholder = match target {
            Target::All => String::new(),
            Target::Robot(..) => format_value(self.settings.bangbang.get(param)),
        };
        text_input(&placeholder, &shown)
            .on_input(move |typed| RobotSettingsMessage::Edit(target, param, typed))
            .on_submit(RobotSettingsMessage::Apply)
            .font(theme::MONO)
            .size(12)
            .padding([2, 6])
            .align_x(alignment::Horizontal::Right)
            .width(Length::Fixed(INPUT_W))
            .style(move |t, s| field_style(t, s, state))
            .into()
    }
}

const INPUT_W: f32 = 80.0;
const RESET_W: f32 = 52.0;

/// The value `target` has now: always one for all robots, a robot's own one or None.
fn current(settings: &RobotSettings, target: Target, param: Param) -> Option<f64> {
    match target {
        Target::All => Some(settings.bangbang.get(param)),
        Target::Robot(team, id) => settings.bangbang.robot(team, id).and_then(|r| r.get(param)),
    }
}

/// A typed value: a number valid for `param`, or (for a robot) empty = the all-robots value.
fn parse(target: Target, param: Param, typed: &str) -> Result<Option<f64>, &'static str> {
    let typed = typed.trim();
    if typed.is_empty() {
        return match target {
            Target::All => Err("needs a value"),
            Target::Robot(..) => Ok(None),
        };
    }
    match typed.parse::<f64>() {
        Ok(v) if param.valid(v) => Ok(Some(v)),
        _ if param == Param::RampAccel => Err("must be a number >= 0 (0 = no ramp)"),
        _ => Err("must be a number > 0"),
    }
}

fn label(param: Param) -> &'static str {
    match param {
        Param::MaxAccel => "Max acceleration",
        Param::MaxSpeed => "Max speed",
        Param::RampAccel => "Command ramp (grip)",
    }
}

fn unit(param: Param) -> &'static str {
    match param {
        Param::MaxAccel => "m/s²",
        Param::MaxSpeed => "m/s",
        Param::RampAccel => "m/s²",
    }
}

fn section_title(title: &str) -> Element<'_, RobotSettingsMessage> {
    text(title).size(11).font(theme::MONO_BOLD).color(theme::MUTED).into()
}

fn group(content: Element<'_, RobotSettingsMessage>) -> Element<'_, RobotSettingsMessage> {
    container(content)
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

/// Shortest text that reads back as the same number.
fn format_value(v: f64) -> String {
    let s = format!("{v}");
    if s.contains('.') || s.contains('e') { s } else { format!("{s}.0") }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::motion::settings::RobotBangBang;

    fn panel() -> RobotSettingsPanel {
        let mut panel = RobotSettingsPanel::default();
        panel.settings.bangbang.robots.push(RobotBangBang {
            team: TeamColor::Blue,
            id: 2,
            max_accel: Some(1.5),
            max_speed: None,
            ramp_accel: None,
        });
        panel
    }

    #[test]
    fn edits_become_motion_commands() {
        let mut panel = panel();
        panel.update(RobotSettingsMessage::Edit(Target::All, Param::MaxSpeed, "4".into()));
        panel.update(RobotSettingsMessage::Edit(Target::Robot(TeamColor::Yellow, 1), Param::MaxAccel, "3.5".into()));
        // Emptying a robot's own value: back to the all-robots one.
        panel.update(RobotSettingsMessage::Edit(Target::Robot(TeamColor::Blue, 2), Param::MaxAccel, "".into()));
        assert_eq!(
            panel.update(RobotSettingsMessage::Apply),
            ["motion all max_speed 4", "motion blue 2 max_accel default", "motion yellow 1 max_accel 3.5"]
        );
    }

    #[test]
    fn invalid_edits_are_not_applied() {
        let mut panel = panel();
        panel.update(RobotSettingsMessage::Edit(Target::All, Param::MaxAccel, "".into()));
        assert!(panel.update(RobotSettingsMessage::Apply).is_empty());
        panel.update(RobotSettingsMessage::Edit(Target::All, Param::MaxAccel, "0".into()));
        assert!(panel.error().is_some());
    }

    #[test]
    fn typing_the_running_value_is_not_an_edit() {
        let mut panel = panel();
        panel.update(RobotSettingsMessage::Edit(Target::Robot(TeamColor::Blue, 2), Param::MaxAccel, "1.5".into()));
        panel.update(RobotSettingsMessage::Edit(Target::Robot(TeamColor::Blue, 2), Param::MaxSpeed, " ".into()));
        assert!(panel.edits.is_empty());
    }

    #[test]
    fn lists_robots_on_the_field_and_robots_with_own_values() {
        let mut panel = panel();
        panel.set_visible([(1, 0), (0, 2), (0, 1)]);
        assert_eq!(panel.robots(), [(TeamColor::Blue, 1), (TeamColor::Blue, 2), (TeamColor::Yellow, 0)]);
    }
}
