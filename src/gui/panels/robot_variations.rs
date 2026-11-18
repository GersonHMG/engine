// gui/panels/robot_variations.rs — Generate variations of the simulator's robot and save them
//
// Pick a base profile (the running simulator, the defaults or a saved one),
// choose which parameters vary and by how much (±% of the base), and generate
// N variations as a Latin hypercube (robot_profiles::generate). Each variation
// is a row of deviation bars, one per varied parameter, so the batch reads at
// a glance. Try one in the running simulator (`sim robot <name> <value>`),
// save the ones worth keeping to robots/<name>.toml, and load saved ones back
// (`sim robot load <name>`).

use iced::alignment::Horizontal;
use iced::widget::{button, checkbox, column, container, pick_list, row, text, text_input, tooltip, Space};
use iced::{Alignment, Background, Border, Color, Element, Length};
use ssl_sim::{RobotConfig, SimConfig};
use std::collections::BTreeMap;
use std::fmt;

use crate::gui::theme::{self, btn, Btn};
use crate::robot_profiles::{self, Spread};

/// One robot parameter: config name, label, column label, unit.
struct Param {
    name: &'static str,
    label: &'static str,
    short: &'static str,
    unit: &'static str,
}

const fn p(name: &'static str, label: &'static str, short: &'static str, unit: &'static str) -> Param {
    Param { name, label, short, unit }
}

const GROUPS: &[(&str, &[Param])] = &[
    (
        "MOTION",
        &[
            p("max_speed", "Max speed", "speed", "m/s"),
            p("max_accel", "Max acceleration", "accel", "m/s²"),
            p("max_omega", "Max angular speed", "ω", "rad/s"),
            p("max_alpha", "Max angular accel.", "α", "rad/s²"),
        ],
    ),
    (
        "TRACTION",
        &[
            p("grip_accel", "Grip", "grip", "m/s²"),
            p("slide_grip", "Grip while sliding", "slide", "0–1"),
            p("slip_spin", "Spin while sliding", "spin", "rad/m"),
        ],
    ),
    (
        "KICKER",
        &[
            p("kick_x_speed", "Flat kick speed", "kickx", "m/s"),
            p("max_kick_speed", "Max kick speed", "kick", "m/s"),
            p("kick_reach", "Kick reach", "reach", "m"),
        ],
    ),
    (
        "BODY",
        &[
            p("radius", "Radius", "r", "m"),
            p("front_distance", "Front distance", "front", "m"),
            p("body_restitution", "Body bounce", "bnc", "0–1"),
            p("front_restitution", "Front bounce", "fbnc", "0–1"),
        ],
    ),
];

fn params() -> impl Iterator<Item = &'static Param> {
    GROUPS.iter().flat_map(|(_, ps)| ps.iter())
}

fn param(name: &str) -> Option<&'static Param> {
    params().find(|p| p.name == name)
}

/// Spreads a preset button sets, in %.
const PRESETS: [(&str, f64); 3] = [("Mild ±5%", 5.0), ("Moderate ±15%", 15.0), ("Wild ±30%", 30.0)];
const MAX_COUNT: usize = 64;

const CELL_W: f32 = 40.0;
const NAME_W: f32 = 120.0;
/// Deviation bars: positive accent, negative orange.
const UP: Color = theme::ACCENT;
const DOWN: Color = theme::ORANGE;

/// Where the variations start from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Base {
    Running,
    Defaults,
    Saved(String),
}

impl fmt::Display for Base {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Base::Running => write!(f, "Running simulator"),
            Base::Defaults => write!(f, "Simulator defaults"),
            Base::Saved(name) => write!(f, "robots/{name}"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum VariationsMessage {
    Base(Base),
    Vary(&'static str, bool),
    Spread(&'static str, String),
    Preset(f64),
    Prefix(String),
    Count(String),
    Seed(String),
    RollSeed,
    Generate,
    Select(usize, bool),
    SelectAll(bool),
    /// Runs the simulator with variation i.
    Try(usize),
    SaveSelected,
    RefreshSaved,
    Load(String),
    UseAsBase(String),
    /// First click asks, second click deletes.
    Delete(String),
}

struct Variation {
    name: String,
    robot: RobotConfig,
    selected: bool,
    saved: bool,
}

/// One press of Generate.
struct Batch {
    base_label: String,
    base: RobotConfig,
    spreads: Vec<Spread>,
    seed: u64,
    items: Vec<Variation>,
}

struct Saved {
    name: String,
    robot: Result<RobotConfig, String>,
}

pub struct RobotVariationsPanel {
    /// What the simulator runs with; None while it is off.
    running: Option<SimConfig>,
    base: Base,
    /// Per parameter: varied?, spread typed in %.
    spreads: BTreeMap<&'static str, (bool, String)>,
    prefix: String,
    count: String,
    seed: String,
    batch: Option<Batch>,
    saved: Vec<Saved>,
    confirm_delete: Option<String>,
    status: Option<Result<String, String>>,
}

impl Default for RobotVariationsPanel {
    fn default() -> Self {
        let spreads = params()
            .map(|p| {
                let on = matches!(p.name, "max_speed" | "max_accel" | "max_omega" | "max_alpha" | "kick_x_speed" | "max_kick_speed");
                (p.name, (on, if on { "15" } else { "10" }.to_string()))
            })
            .collect();
        let mut panel = Self {
            running: None,
            base: Base::Running,
            spreads,
            prefix: "var".to_string(),
            count: "8".to_string(),
            seed: "1".to_string(),
            batch: None,
            saved: Vec::new(),
            confirm_delete: None,
            status: None,
        };
        panel.refresh_saved();
        panel
    }
}

impl RobotVariationsPanel {
    /// The engine sent the simulator's parameters (None = simulator off).
    pub fn set_config(&mut self, config: Option<SimConfig>) {
        self.running = config;
    }

    fn refresh_saved(&mut self) {
        self.saved = robot_profiles::list()
            .into_iter()
            .map(|name| Saved { robot: robot_profiles::load(&name), name })
            .collect();
        if let Base::Saved(name) = &self.base {
            if !self.saved.iter().any(|s| &s.name == name) {
                self.base = Base::Running;
            }
        }
    }

    /// The base profile and whether it is the one asked for (the running
    /// simulator falls back to the defaults while it is off).
    fn base_profile(&self) -> (RobotConfig, String) {
        match &self.base {
            Base::Running => match &self.running {
                Some(c) => (c.robot.clone(), "running simulator".into()),
                None => (RobotConfig::default(), "simulator defaults (SIM is off)".into()),
            },
            Base::Defaults => (RobotConfig::default(), "simulator defaults".into()),
            Base::Saved(name) => match self.saved.iter().find(|s| &s.name == name).map(|s| &s.robot) {
                Some(Ok(robot)) => (robot.clone(), format!("robots/{name}")),
                _ => (RobotConfig::default(), "simulator defaults".into()),
            },
        }
    }

    fn running_robot(&self) -> Option<&RobotConfig> {
        self.running.as_ref().map(|c| &c.robot)
    }

    /// The spreads to generate with, or why they are not usable.
    fn chosen_spreads(&self) -> Result<Vec<Spread>, String> {
        let mut out = Vec::new();
        for p in params() {
            let (on, typed) = &self.spreads[p.name];
            if !on {
                continue;
            }
            let pct: f64 = typed.trim().parse().map_err(|_| format!("{}: spread is not a number", p.label))?;
            if !(0.0..=100.0).contains(&pct) {
                return Err(format!("{}: spread must be 0–100 %", p.label));
            }
            out.push(Spread { name: p.name, fraction: pct / 100.0 });
        }
        if out.is_empty() {
            return Err("Tick at least one parameter to vary".into());
        }
        Ok(out)
    }

    fn generate(&mut self) -> Result<String, String> {
        let spreads = self.chosen_spreads()?;
        let count: usize = self.count.trim().parse().map_err(|_| "Count is not a whole number".to_string())?;
        if !(1..=MAX_COUNT).contains(&count) {
            return Err(format!("Count must be 1–{MAX_COUNT}"));
        }
        let seed: u64 = self.seed.trim().parse().map_err(|_| "Seed is not a whole number".to_string())?;
        let prefix = self.prefix.trim().to_string();
        if !robot_profiles::valid_name(&prefix) {
            return Err("Name prefix: letters, digits, - and _ only".into());
        }

        // Number after the saved profiles with this prefix, so saving never overwrites.
        let first = robot_profiles::list()
            .iter()
            .filter_map(|n| n.strip_prefix(&format!("{prefix}-"))?.parse::<usize>().ok())
            .max()
            .map_or(1, |n| n + 1);
        let width = (first + count - 1).to_string().len().max(2);

        let (base, base_label) = self.base_profile();
        let items = robot_profiles::generate(&base, &spreads, count, seed)
            .into_iter()
            .enumerate()
            .map(|(i, robot)| Variation {
                name: format!("{prefix}-{:0width$}", first + i),
                robot,
                selected: false,
                saved: false,
            })
            .collect();
        self.batch = Some(Batch { base_label, base, spreads, seed, items });
        Ok(format!("Generated {count} variations"))
    }

    fn save_selected(&mut self) -> Result<String, String> {
        let Some(batch) = &mut self.batch else { return Err("Nothing generated yet".into()) };
        let spreads: Vec<String> = batch
            .spreads
            .iter()
            .map(|s| format!("{} ±{}%", s.name, trim_number(s.fraction * 100.0)))
            .collect();
        let note = format!("Variation of {} (seed {}): {}", batch.base_label, batch.seed, spreads.join(", "));
        let mut saved = 0;
        for item in batch.items.iter_mut().filter(|i| i.selected) {
            robot_profiles::save(&item.name, &item.robot, &note)?;
            item.saved = true;
            item.selected = false;
            saved += 1;
        }
        self.refresh_saved();
        Ok(format!("Saved {saved} profile{} to {}/", if saved == 1 { "" } else { "s" }, robot_profiles::DIR))
    }

    /// Handles a message; returns the console commands to run.
    pub fn update(&mut self, message: VariationsMessage) -> Vec<String> {
        if !matches!(message, VariationsMessage::Delete(_)) {
            self.confirm_delete = None;
        }
        match message {
            VariationsMessage::Base(base) => self.base = base,
            VariationsMessage::Vary(name, on) => {
                if let Some(entry) = self.spreads.get_mut(name) {
                    entry.0 = on;
                }
            }
            VariationsMessage::Spread(name, typed) => {
                if let Some(entry) = self.spreads.get_mut(name) {
                    entry.1 = typed;
                }
            }
            VariationsMessage::Preset(pct) => {
                for entry in self.spreads.values_mut().filter(|e| e.0) {
                    entry.1 = trim_number(pct);
                }
            }
            VariationsMessage::Prefix(s) => self.prefix = s,
            VariationsMessage::Count(s) => self.count = s,
            VariationsMessage::Seed(s) => self.seed = s,
            VariationsMessage::RollSeed => {
                let nanos = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(7, |d| d.subsec_nanos());
                self.seed = (nanos % 100_000).to_string();
            }
            VariationsMessage::Generate => self.status = Some(self.generate()),
            VariationsMessage::Select(i, on) => {
                if let Some(item) = self.batch.as_mut().and_then(|b| b.items.get_mut(i)) {
                    item.selected = on;
                }
            }
            VariationsMessage::SelectAll(on) => {
                for item in self.batch.iter_mut().flat_map(|b| b.items.iter_mut()) {
                    item.selected = on && !item.saved;
                }
            }
            VariationsMessage::Try(i) => {
                let (Some(running), Some(item)) = (self.running_robot(), self.batch.as_ref().and_then(|b| b.items.get(i))) else {
                    return Vec::new();
                };
                let (commands, status) = (commands_to(running, &item.robot), format!("Simulator now runs {}", item.name));
                self.status = Some(Ok(status));
                return commands;
            }
            VariationsMessage::SaveSelected => self.status = Some(self.save_selected()),
            VariationsMessage::RefreshSaved => self.refresh_saved(),
            VariationsMessage::Load(name) => {
                if self.running.is_none() {
                    return Vec::new();
                }
                self.status = Some(Ok(format!("Simulator now runs robots/{name}")));
                return vec![format!("sim robot load {name}")];
            }
            VariationsMessage::UseAsBase(name) => self.base = Base::Saved(name),
            VariationsMessage::Delete(name) => {
                if self.confirm_delete.as_deref() == Some(&name) {
                    self.confirm_delete = None;
                    self.status = Some(robot_profiles::delete(&name).map(|()| format!("Deleted robots/{name}.toml")));
                    self.refresh_saved();
                    if let Some(batch) = &mut self.batch {
                        for item in batch.items.iter_mut().filter(|i| i.name == name) {
                            item.saved = false;
                        }
                    }
                } else {
                    self.confirm_delete = Some(name);
                }
            }
        }
        Vec::new()
    }

    // --- View ---

    pub fn view(&self) -> Element<'_, VariationsMessage> {
        let mut content = column![self.view_base(), self.view_spreads(), self.view_generate()].spacing(10);
        if let Some(status) = &self.status {
            content = content.push(status_line(status));
        }
        if let Some(batch) = &self.batch {
            content = content.push(self.view_batch(batch));
        }
        content = content.push(self.view_saved());
        container(content).padding(10).width(Length::Fill).into()
    }

    fn view_base(&self) -> Element<'_, VariationsMessage> {
        let mut options = vec![Base::Running, Base::Defaults];
        options.extend(self.saved.iter().filter(|s| s.robot.is_ok()).map(|s| Base::Saved(s.name.clone())));
        let picker = pick_list(options, Some(self.base.clone()), VariationsMessage::Base)
            .text_size(12)
            .padding([3, 8])
            .width(Length::Fixed(220.0));

        let (base, label) = self.base_profile();
        let mut chips = row![].spacing(6);
        for name in ["max_speed", "max_accel", "grip_accel", "kick_x_speed", "radius"] {
            let p = param(name).expect("known parameter");
            chips = chips.push(chip(p.short, &format_value(robot_profiles::value(&base, name).unwrap_or(0.0)), p.unit));
        }
        let fallback = (self.base == Base::Running && self.running.is_none())
            .then(|| text(format!("Using {label}")).size(10).color(theme::ORANGE));

        group(
            column![
                row![section_title("BASE PROFILE"), Space::new().width(Length::Fill), picker].align_y(Alignment::Center),
                chips,
            ]
            .push(fallback.map_or_else(|| Element::from(Space::new().height(0)), Element::from))
            .spacing(8)
            .into(),
        )
    }

    fn view_spreads(&self) -> Element<'_, VariationsMessage> {
        let (base, _) = self.base_profile();
        let mut presets = row![].spacing(4);
        for (label, pct) in PRESETS {
            presets = presets.push(small_button(label, Btn::Ghost, Some(VariationsMessage::Preset(pct))));
        }
        let varied = self.spreads.values().filter(|e| e.0).count();

        let block = |title: &'static str, ps: &'static [Param]| -> Element<'_, VariationsMessage> {
            let mut col = column![text(title).size(10).font(theme::MONO_BOLD).color(theme::MUTED)].spacing(3);
            for p in ps {
                col = col.push(self.spread_row(p, &base));
            }
            col.into()
        };
        let left = column![block(GROUPS[0].0, GROUPS[0].1), block(GROUPS[1].0, GROUPS[1].1)].spacing(10).width(Length::Fill);
        let right = column![block(GROUPS[2].0, GROUPS[2].1), block(GROUPS[3].0, GROUPS[3].1)].spacing(10).width(Length::Fill);

        group(
            column![
                row![
                    section_title("SPREAD"),
                    text(format!("{varied} of {} parameters vary", params().count())).size(11).color(theme::MUTED),
                    Space::new().width(Length::Fill),
                    presets,
                ]
                .spacing(10)
                .align_y(Alignment::Center),
                row![left, container(Space::new().width(1).height(Length::Fill)).style(theme::bar(theme::DIVIDER)), right]
                    .spacing(14)
                    .height(Length::Shrink),
                text("Each variation draws every ticked parameter from base ± spread. The draws form a Latin hypercube: every slice of each range is used once, so a few variations already cover it.")
                    .size(10)
                    .color(theme::MUTED),
            ]
            .spacing(8)
            .into(),
        )
    }

    fn spread_row(&self, p: &'static Param, base: &RobotConfig) -> Element<'_, VariationsMessage> {
        let (on, typed) = &self.spreads[p.name];
        let on = *on;
        let base_value = robot_profiles::value(base, p.name).unwrap_or(0.0);
        let pct = typed.trim().parse::<f64>().ok().filter(|v| (0.0..=100.0).contains(v));

        let range: Element<'_, VariationsMessage> = if !on {
            text(format_value(base_value)).size(11).font(theme::MONO).color(theme::MUTED).into()
        } else if base_value == 0.0 {
            tooltip(
                text("0 · fixed").size(11).font(theme::MONO).color(theme::ORANGE),
                container(text("A spread of a 0 base is 0. Give the base a value first.").size(11)).padding([4, 8]).style(theme::tooltip_box),
                tooltip::Position::Top,
            )
            .into()
        } else if let Some(pct) = pct {
            let (mut lo, mut hi) = (base_value * (1.0 - pct / 100.0), base_value * (1.0 + pct / 100.0));
            lo = lo.max(0.0);
            if robot_profiles::is_fraction(p.name) {
                hi = hi.min(1.0);
            }
            text(format!("{} – {}", format_value(lo), format_value(hi))).size(11).font(theme::MONO).color(theme::TEXT_2).into()
        } else {
            text("—").size(11).color(theme::RED_FG).into()
        };

        let name = p.name;
        let invalid = on && pct.is_none();
        let input = text_input("", typed)
            .on_input(move |s| VariationsMessage::Spread(name, s))
            .on_submit(VariationsMessage::Generate)
            .font(theme::MONO)
            .size(11)
            .padding([1, 4])
            .align_x(Horizontal::Right)
            .width(Length::Fixed(38.0))
            .style(move |t, s| {
                let mut style = theme::input(t, s);
                if invalid {
                    style.border.color = theme::RED;
                    style.value = theme::RED_FG;
                } else if !on {
                    style.value = theme::MUTED;
                }
                style
            });

        row![
            checkbox(on).on_toggle(move |v| VariationsMessage::Vary(name, v)).style(theme::check).size(13),
            text(p.label).size(12).color(if on { theme::TEXT } else { theme::MUTED }).width(Length::Fill),
            container(range).width(Length::Fixed(104.0)).align_x(Horizontal::Right),
            text(p.unit).size(10).font(theme::MONO).color(theme::MUTED).width(Length::Fixed(36.0)),
            text("±").size(11).color(if on { theme::TEXT_2 } else { theme::MUTED }),
            input,
            text("%").size(11).color(theme::MUTED),
        ]
        .spacing(5)
        .align_y(Alignment::Center)
        .into()
    }

    fn view_generate(&self) -> Element<'_, VariationsMessage> {
        let field = |label: &'static str, value: &str, width: f32, on: fn(String) -> VariationsMessage| {
            row![
                text(label).size(11).color(theme::MUTED),
                text_input("", value)
                    .on_input(on)
                    .on_submit(VariationsMessage::Generate)
                    .font(theme::MONO)
                    .size(12)
                    .padding([3, 6])
                    .width(Length::Fixed(width))
                    .style(theme::input),
            ]
            .spacing(6)
            .align_y(Alignment::Center)
        };
        let roll = tooltip(
            button(container(theme::icon(theme::Icon::Dice, 14.0, Some(theme::TEXT_2))).center(Length::Fill))
                .width(24)
                .height(24)
                .padding(0)
                .style(btn(Btn::Ghost))
                .on_press(VariationsMessage::RollSeed),
            container(text("New random seed").size(11)).padding([4, 8]).style(theme::tooltip_box),
            tooltip::Position::Top,
        );
        let ready = self.chosen_spreads().is_ok();
        let go = button(row![theme::icon(theme::Icon::Dice, 14.0, Some(theme::btn_fg(Btn::Play, ready))), text("Generate").size(12)].spacing(6).align_y(Alignment::Center))
            .padding([5, 14])
            .style(btn(Btn::Play))
            .on_press_maybe(ready.then_some(VariationsMessage::Generate));

        group(
            row![
                field("Name", &self.prefix, 90.0, VariationsMessage::Prefix),
                field("Count", &self.count, 44.0, VariationsMessage::Count),
                row![field("Seed", &self.seed, 64.0, VariationsMessage::Seed), roll].spacing(2).align_y(Alignment::Center),
                Space::new().width(Length::Fill),
                go,
            ]
            .spacing(14)
            .align_y(Alignment::Center)
            .into(),
        )
    }

    fn view_batch<'a>(&'a self, batch: &'a Batch) -> Element<'a, VariationsMessage> {
        let running = self.running_robot();
        let selected = batch.items.iter().filter(|i| i.selected).count();
        let unsaved = batch.items.iter().filter(|i| !i.saved).count();

        // Column heads: one per varied parameter.
        let mut head = row![Space::new().width(Length::Fixed(18.0)), text("").width(Length::Fixed(NAME_W))].spacing(6);
        let mut cells_head = row![].spacing(0);
        for s in &batch.spreads {
            let p = param(s.name).expect("known parameter");
            cells_head = cells_head.push(
                tooltip(
                    text(p.short).size(10).font(theme::MONO).color(theme::MUTED).width(Length::Fixed(CELL_W)).align_x(Horizontal::Center),
                    container(text(format!("{} ±{}%", p.label, trim_number(s.fraction * 100.0))).size(11)).padding([4, 8]).style(theme::tooltip_box),
                    tooltip::Position::Top,
                ),
            );
        }
        head = head.push(cells_head);

        let mut rows = column![head].spacing(1);
        for (i, item) in batch.items.iter().enumerate() {
            rows = rows.push(self.variation_row(batch, i, item, running));
        }

        let legend = row![
            legend_swatch(DOWN, "below base"),
            legend_swatch(UP, "above base"),
            text("· full bar = the whole spread").size(10).color(theme::MUTED),
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        let footer = row![
            small_button("All", Btn::Ghost, (unsaved > 0).then_some(VariationsMessage::SelectAll(true))),
            small_button("None", Btn::Ghost, (selected > 0).then_some(VariationsMessage::SelectAll(false))),
            legend,
            Space::new().width(Length::Fill),
            button(text(if selected == 0 { "Save selected".to_string() } else { format!("Save {selected} selected") }).size(12))
                .padding([4, 12])
                .style(btn(Btn::Toggle(selected > 0)))
                .on_press_maybe((selected > 0).then_some(VariationsMessage::SaveSelected)),
        ]
        .spacing(6)
        .align_y(Alignment::Center);

        group(
            column![
                row![
                    section_title("VARIATIONS"),
                    text(format!("{} from {} · seed {}", batch.items.len(), batch.base_label, batch.seed)).size(11).color(theme::MUTED),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
                rows,
                footer,
            ]
            .spacing(8)
            .into(),
        )
    }

    fn variation_row<'a>(&'a self, batch: &'a Batch, i: usize, item: &'a Variation, running: Option<&RobotConfig>) -> Element<'a, VariationsMessage> {
        let is_running = running == Some(&item.robot);
        let mut select = checkbox(item.selected).style(theme::check).size(13);
        if !item.saved {
            select = select.on_toggle(move |v| VariationsMessage::Select(i, v));
        }

        let mut name = row![text(item.name.clone()).size(12).font(theme::MONO).color(theme::TEXT)].spacing(5).align_y(Alignment::Center);
        if is_running {
            name = name.push(theme::dot(theme::GREEN, 6.0));
        }
        if item.saved {
            name = name.push(text("saved").size(9).font(theme::MONO).color(theme::GREEN));
        }

        let mut cells = row![].spacing(0).align_y(Alignment::Center);
        for s in &batch.spreads {
            let p = param(s.name).expect("known parameter");
            let base = robot_profiles::value(&batch.base, s.name).unwrap_or(0.0);
            let value = robot_profiles::value(&item.robot, s.name).unwrap_or(0.0);
            let rel = if base == 0.0 { 0.0 } else { value / base - 1.0 };
            let tip = format!(
                "{}: {} {} ({}{:.1}%)",
                p.label,
                format_value(value),
                p.unit,
                if rel >= 0.0 { "+" } else { "−" },
                rel.abs() * 100.0
            );
            cells = cells.push(tooltip(
                deviation_bar(if s.fraction > 0.0 { rel / s.fraction } else { 0.0 }),
                container(text(tip).size(11).font(theme::MONO)).padding([4, 8]).style(theme::tooltip_box),
                tooltip::Position::Top,
            ));
        }

        let try_it = button(text(if is_running { "Running" } else { "Try" }).size(11))
            .padding([2, 10])
            .style(btn(if is_running { Btn::Toggle(true) } else { Btn::Neutral }))
            .on_press_maybe((running.is_some() && !is_running).then_some(VariationsMessage::Try(i)));

        let line = row![
            container(select).width(Length::Fixed(18.0)),
            container(name).width(Length::Fixed(NAME_W)),
            cells,
            Space::new().width(Length::Fill),
            try_it,
        ]
        .spacing(6)
        .align_y(Alignment::Center);

        let highlight = item.selected || is_running;
        container(line)
            .padding([3, 6])
            .width(Length::Fill)
            .style(move |_| container::Style {
                background: Some(Background::Color(if highlight { theme::BTN } else { Color::TRANSPARENT })),
                border: Border { radius: 4.0.into(), ..Default::default() },
                ..Default::default()
            })
            .into()
    }

    fn view_saved(&self) -> Element<'_, VariationsMessage> {
        let running = self.running_robot();
        let mut list = column![].spacing(1);
        if self.saved.is_empty() {
            list = list.push(
                text(format!("No saved profiles yet. Saved variations go to {}/ in the working directory.", robot_profiles::DIR))
                    .size(11)
                    .color(theme::MUTED),
            );
        }
        for s in &self.saved {
            let is_running = matches!((&s.robot, running), (Ok(r), Some(run)) if r == run);
            let mut name = row![text(s.name.clone()).size(12).font(theme::MONO)].spacing(5).align_y(Alignment::Center);
            if is_running {
                name = name.push(theme::dot(theme::GREEN, 6.0));
            }
            let summary: Element<'_, VariationsMessage> = match &s.robot {
                Ok(r) => text(format!(
                    "{} m/s · {} m/s² · grip {} · kick {} m/s",
                    format_value(r.max_speed),
                    format_value(r.max_accel),
                    if r.grip_accel == 0.0 { "perfect".to_string() } else { format_value(r.grip_accel) },
                    format_value(r.kick_x_speed),
                ))
                .size(11)
                .font(theme::MONO)
                .color(theme::TEXT_2)
                .into(),
                Err(e) => text(e.clone()).size(11).color(theme::RED_FG).into(),
            };
            let ok = s.robot.is_ok();
            let confirming = self.confirm_delete.as_deref() == Some(s.name.as_str());
            let line = row![
                container(name).width(Length::Fixed(NAME_W + 24.0)),
                container(summary).width(Length::Fill),
                small_button("Load", Btn::Neutral, (ok && running.is_some() && !is_running).then(|| VariationsMessage::Load(s.name.clone()))),
                small_button("Use as base", Btn::Ghost, ok.then(|| VariationsMessage::UseAsBase(s.name.clone()))),
                button(text(if confirming { "Delete?" } else { "Delete" }).size(11))
                    .padding([2, 8])
                    .style(btn(if confirming { Btn::Rec(true) } else { Btn::Ghost }))
                    .on_press(VariationsMessage::Delete(s.name.clone())),
            ]
            .spacing(6)
            .align_y(Alignment::Center);
            list = list.push(container(line).padding([3, 6]));
        }

        let hint = if self.running.is_some() {
            "Load and Try change the running simulator only; Save in Simulator parameters keeps it in simulator.toml."
        } else {
            "Turn on SIM in the toolbar to try or load a profile."
        };

        group(
            column![
                row![
                    section_title("SAVED PROFILES"),
                    text(format!("{}/ · {}", robot_profiles::DIR, self.saved.len())).size(11).color(theme::MUTED),
                    Space::new().width(Length::Fill),
                    small_button("Reload", Btn::Ghost, Some(VariationsMessage::RefreshSaved)),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
                list,
                text(hint).size(10).color(theme::MUTED),
            ]
            .spacing(8)
            .into(),
        )
    }
}

/// Console commands that turn `from` into `to`.
fn commands_to(from: &RobotConfig, to: &RobotConfig) -> Vec<String> {
    params()
        .filter_map(|p| {
            let v = robot_profiles::value(to, p.name)?;
            (robot_profiles::value(from, p.name) != Some(v)).then(|| format!("sim robot {} {v}", p.name))
        })
        .collect()
}

// --- Pieces ---

/// A bar from the center: `t` = -1 fills the left half, +1 the right half.
fn deviation_bar<'a>(t: f64) -> Element<'a, VariationsMessage> {
    const HALF: f32 = 15.0;
    const H: f32 = 10.0;
    let len = (t.clamp(-1.0, 1.0).abs() as f32 * HALF).max(1.5);
    let bar = |c: Color, w: f32| {
        container(Space::new().width(w).height(H)).style(move |_| container::Style {
            background: Some(Background::Color(c)),
            border: Border { radius: 2.0.into(), ..Default::default() },
            ..Default::default()
        })
    };
    let side = |filled: Option<Color>, align: Horizontal| {
        let inner: Element<'a, VariationsMessage> = match filled {
            Some(c) => bar(c, len).into(),
            None => Space::new().width(0).height(H).into(),
        };
        container(inner).width(Length::Fixed(HALF)).align_x(align)
    };
    let (left, right) = if t < 0.0 { (Some(DOWN), None) } else { (None, Some(UP)) };
    container(
        row![
            side(left, Horizontal::Right),
            container(Space::new().width(1).height(H + 4.0)).style(theme::bar(theme::BTN_BORDER)),
            side(right, Horizontal::Left),
        ]
        .align_y(Alignment::Center),
    )
    .width(Length::Fixed(CELL_W))
    .center_x(Length::Fixed(CELL_W))
    .into()
}

fn legend_swatch<'a>(c: Color, label: &'a str) -> Element<'a, VariationsMessage> {
    row![
        container(Space::new().width(10).height(8)).style(move |_| container::Style {
            background: Some(Background::Color(c)),
            border: Border { radius: 2.0.into(), ..Default::default() },
            ..Default::default()
        }),
        text(label).size(10).color(theme::MUTED),
    ]
    .spacing(4)
    .align_y(Alignment::Center)
    .into()
}

fn chip<'a>(label: &'a str, value: &str, unit: &'a str) -> Element<'a, VariationsMessage> {
    container(
        row![
            text(label).size(10).font(theme::MONO).color(theme::MUTED),
            text(value.to_string()).size(12).font(theme::MONO_BOLD).color(theme::TEXT),
            text(unit).size(10).font(theme::MONO).color(theme::MUTED),
        ]
        .spacing(4)
        .align_y(Alignment::Center),
    )
    .padding([3, 8])
    .style(|_| container::Style {
        background: Some(Background::Color(theme::INPUT)),
        border: Border { color: theme::DIVIDER, width: 1.0, radius: 10.0.into() },
        ..Default::default()
    })
    .into()
}

fn status_line(status: &Result<String, String>) -> Element<'_, VariationsMessage> {
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

fn small_button(label: &str, kind: Btn, message: Option<VariationsMessage>) -> Element<'_, VariationsMessage> {
    button(text(label).size(11)).padding([2, 8]).style(btn(kind)).on_press_maybe(message).into()
}

fn section_title(title: &str) -> Element<'_, VariationsMessage> {
    text(title).size(11).font(theme::MONO_BOLD).color(theme::MUTED).into()
}

fn group(content: Element<'_, VariationsMessage>) -> Element<'_, VariationsMessage> {
    container(content).padding([10, 12]).width(Length::Fill).style(theme::group).into()
}

/// Up to 4 significant digits, no trailing zeros.
fn format_value(v: f64) -> String {
    if v == 0.0 {
        return "0".into();
    }
    let decimals = (3 - v.abs().log10().floor() as i32).clamp(0, 6) as usize;
    trim_number_to(v, decimals)
}

fn trim_number(v: f64) -> String {
    trim_number_to(v, 2)
}

fn trim_number_to(v: f64, decimals: usize) -> String {
    let s = format!("{v:.decimals$}");
    if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.').to_string() } else { s }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_parameter_exists_in_the_profile() {
        let robot = RobotConfig::default();
        for p in params() {
            assert!(robot_profiles::value(&robot, p.name).is_some(), "{}", p.name);
        }
        assert_eq!(params().count(), serde_json::to_value(&robot).unwrap().as_object().unwrap().len());
    }

    #[test]
    fn generates_the_requested_batch() {
        let mut panel = RobotVariationsPanel::default();
        panel.prefix = "zz_test_never_saved".into();
        panel.count = "5".into();
        panel.update(VariationsMessage::Generate);
        let batch = panel.batch.as_ref().expect("generated");
        assert_eq!(batch.items.len(), 5);
        assert_eq!(batch.items[0].name, "zz_test_never_saved-01");
        assert!(batch.spreads.iter().any(|s| s.name == "max_speed"));
    }

    #[test]
    fn rejects_bad_inputs() {
        let mut panel = RobotVariationsPanel::default();
        panel.count = "0".into();
        panel.update(VariationsMessage::Generate);
        assert!(matches!(panel.status, Some(Err(_))));
        assert!(panel.batch.is_none());

        let mut panel = RobotVariationsPanel::default();
        for entry in panel.spreads.values_mut() {
            entry.0 = false;
        }
        panel.update(VariationsMessage::Generate);
        assert!(matches!(&panel.status, Some(Err(e)) if e.contains("at least one")));
    }

    #[test]
    fn trying_a_variation_sends_only_what_changes() {
        let mut panel = RobotVariationsPanel::default();
        panel.set_config(Some(SimConfig::default()));
        panel.prefix = "zz_test_never_saved".into();
        panel.update(VariationsMessage::Generate);
        let commands = panel.update(VariationsMessage::Try(0));
        assert!(!commands.is_empty());
        assert!(commands.iter().all(|c| c.starts_with("sim robot ")));
        assert!(!commands.iter().any(|c| c.contains("radius")), "radius does not vary by default: {commands:?}");
    }

    #[test]
    fn formats_numbers_compactly() {
        assert_eq!(format_value(5.0), "5");
        assert_eq!(format_value(3.456789), "3.457");
        assert_eq!(format_value(0.0734), "0.0734");
        assert_eq!(format_value(40.0), "40");
    }
}
