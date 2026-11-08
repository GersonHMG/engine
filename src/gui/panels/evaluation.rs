// gui/panels/evaluation.rs — Is the Lua change better than the baseline?
//
// Runs every chosen scenario N times for the working tree and for a git commit (HEAD by
// default) with lua/tests/evaluate.lua (src/evaluation), then shows the paired comparison:
// a verdict (exact McNemar on the paired episodes), a table per scenario, and plots made
// with gui/plot: win rates with Wilson intervals, the difference per scenario with
// bootstrap intervals (a forest plot), outcome shares, time to goal, and the cumulative
// goal curve. The last saved evaluation (sessions/eval/) is shown when the panel opens.

use iced::alignment::Horizontal;
use iced::widget::{button, checkbox, column, container, progress_bar, row, text, text_input, Space};
use iced::{Alignment, Background, Border, Color, Element, Length};
use std::path::PathBuf;

use crate::evaluation::runner::{Progress, Run};
use crate::evaluation::stats::{self, Comparison, Summary, Verdict};
use crate::evaluation::{self, EvalSettings, Evaluation};
use crate::gui::plot::chart::horizontal_height;
use crate::gui::plot::{Axis, CategoryChart, Interval, Layer, RefLine, Series, SeriesKind, TickFormat, XyChart, PALETTE};
use crate::gui::theme::{self, btn, Btn};

const BASELINE: Color = PALETTE[1];
const CANDIDATE: Color = PALETTE[0];
const ROW: f32 = 22.0;
const LABEL_W: f32 = 150.0;

#[derive(Debug, Clone)]
pub enum EvaluationMessage {
    Entry(String),
    BaselineRef(String),
    Repeats(String),
    EpisodeTime(String),
    Jitter(String),
    Seed(String),
    ToggleScenario(usize, bool),
    AllScenarios(bool),
    RefreshScenarios,
    Run,
    Cancel,
    LoadLatest,
}

/// A finished evaluation, its statistics and its plots (built once).
struct Shown {
    evaluation: Evaluation,
    summary: Summary,
    path: Option<PathBuf>,
    win_rates: CategoryChart,
    differences: CategoryChart,
    outcomes: CategoryChart,
    times: CategoryChart,
    curve: XyChart,
}

pub struct EvaluationPanel {
    root: PathBuf,
    entry: String,
    baseline_ref: String,
    repeats: String,
    episode_time: String,
    jitter: String,
    seed: String,
    scenarios: Vec<(String, bool)>,
    run: Option<Run>,
    progress: Progress,
    shown: Option<Shown>,
    error: Option<String>,
}

impl Default for EvaluationPanel {
    fn default() -> Self {
        let root = std::env::current_dir().unwrap_or_default();
        let defaults = EvalSettings::default();
        let mut panel = Self {
            scenarios: evaluation::scenario_names(&root).into_iter().map(|n| (n, true)).collect(),
            root,
            entry: defaults.entry,
            baseline_ref: defaults.baseline_ref,
            repeats: defaults.repeats.to_string(),
            episode_time: defaults.episode_time.to_string(),
            jitter: defaults.jitter.to_string(),
            seed: defaults.seed.to_string(),
            run: None,
            progress: Progress::default(),
            shown: None,
            error: None,
        };
        panel.load_latest();
        panel
    }
}

impl EvaluationPanel {
    /// Polls the running evaluation (call every frame).
    pub fn tick(&mut self) {
        let Some(run) = &self.run else { return };
        self.progress = run.progress();
        if let Some(result) = self.progress.result.take() {
            self.run = None;
            match result {
                Ok((evaluation, path)) => {
                    self.error = None;
                    self.shown = Some(Shown::new(evaluation, Some(path)));
                }
                Err(e) => self.error = Some(e),
            }
        }
    }

    pub fn update(&mut self, message: EvaluationMessage) {
        match message {
            EvaluationMessage::Entry(v) => self.entry = v,
            EvaluationMessage::BaselineRef(v) => self.baseline_ref = v,
            EvaluationMessage::Repeats(v) => self.repeats = v,
            EvaluationMessage::EpisodeTime(v) => self.episode_time = v,
            EvaluationMessage::Jitter(v) => self.jitter = v,
            EvaluationMessage::Seed(v) => self.seed = v,
            EvaluationMessage::ToggleScenario(i, on) => {
                if let Some(s) = self.scenarios.get_mut(i) {
                    s.1 = on;
                }
            }
            EvaluationMessage::AllScenarios(on) => self.scenarios.iter_mut().for_each(|s| s.1 = on),
            EvaluationMessage::RefreshScenarios => {
                let chosen: Vec<String> = self.scenarios.iter().filter(|s| s.1).map(|s| s.0.clone()).collect();
                let all = chosen.len() == self.scenarios.len();
                self.scenarios = evaluation::scenario_names(&self.root)
                    .into_iter()
                    .map(|n| {
                        let on = all || chosen.contains(&n);
                        (n, on)
                    })
                    .collect();
            }
            EvaluationMessage::Run => match self.settings() {
                Ok(settings) => {
                    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("engine"));
                    self.error = None;
                    self.progress = Progress { phase: "Starting".into(), ..Progress::default() };
                    self.run = Some(Run::start(settings, self.root.clone(), exe));
                }
                Err(e) => self.error = Some(e),
            },
            EvaluationMessage::Cancel => {
                if let Some(run) = &self.run {
                    run.cancel();
                }
            }
            EvaluationMessage::LoadLatest => self.load_latest(),
        }
    }

    fn load_latest(&mut self) {
        let Some(path) = evaluation::latest_saved(&self.root) else {
            return;
        };
        match Evaluation::load(&path) {
            Ok(e) => {
                let s = &e.settings;
                self.entry = s.entry.clone();
                self.baseline_ref = s.baseline_ref.clone();
                self.repeats = s.repeats.to_string();
                self.episode_time = s.episode_time.to_string();
                self.jitter = s.jitter.to_string();
                self.seed = s.seed.to_string();
                for (name, on) in &mut self.scenarios {
                    *on = s.scenarios.contains(name);
                }
                self.shown = Some(Shown::new(e, Some(path)));
            }
            Err(e) => self.error = Some(e),
        }
    }

    /// The settings typed in the form, or what is wrong with them.
    fn settings(&self) -> Result<EvalSettings, String> {
        let number = |label: &str, typed: &str, min: f64| -> Result<f64, String> {
            typed.trim().parse::<f64>().ok().filter(|v| v.is_finite() && *v >= min).ok_or_else(|| format!("{label} must be a number >= {min}"))
        };
        let scenarios: Vec<String> = self.scenarios.iter().filter(|s| s.1).map(|s| s.0.clone()).collect();
        if scenarios.is_empty() {
            return Err("choose at least one scenario".into());
        }
        if self.entry.trim().is_empty() || self.baseline_ref.trim().is_empty() {
            return Err("entry script and baseline revision are needed".into());
        }
        Ok(EvalSettings {
            entry: self.entry.trim().replace('\\', "/"),
            baseline_ref: self.baseline_ref.trim().to_string(),
            repeats: number("episodes per scenario", &self.repeats, 1.0)? as u32,
            episode_time: number("episode time", &self.episode_time, 1.0)?,
            jitter: number("jitter", &self.jitter, 0.0)?,
            seed: number("seed", &self.seed, 0.0)? as u64,
            scenarios,
        })
    }

    pub fn view(&self) -> Element<'_, EvaluationMessage> {
        let mut content = column![self.view_setup(), self.view_run()].spacing(10);
        if let Some(e) = &self.error {
            content = content.push(
                container(text(e.clone()).size(11).font(theme::MONO).color(theme::RED_FG))
                    .padding([6, 8])
                    .width(Length::Fill)
                    .style(|_| container::Style { background: Some(Background::Color(theme::RED_DIM)), border: Border { radius: 4.0.into(), ..Default::default() }, ..Default::default() }),
            );
        }
        if let Some(shown) = &self.shown {
            content = content.push(shown.view());
        }
        container(content).padding(10).width(Length::Fill).into()
    }

    fn view_setup(&self) -> Element<'_, EvaluationMessage> {
        let enabled = self.run.is_none();
        let field = |label: &'static str, hint: &'static str, value: &str, on: fn(String) -> EvaluationMessage| {
            let input = text_input(hint, value).size(12).style(theme::input).width(Length::Fill);
            let input = if enabled { input.on_input(on) } else { input };
            row![text(label).size(12).color(theme::TEXT_2).width(Length::Fixed(LABEL_W)), input].spacing(8).align_y(Alignment::Center)
        };
        let fields = column![
            field("Entry script", "lua/run_ai.lua", &self.entry, EvaluationMessage::Entry),
            field("Baseline (git revision)", "HEAD", &self.baseline_ref, EvaluationMessage::BaselineRef),
            field("Episodes per scenario", "10", &self.repeats, EvaluationMessage::Repeats),
            field("Episode time (s)", "15", &self.episode_time, EvaluationMessage::EpisodeTime),
            field("Start jitter (m)", "0.05", &self.jitter, EvaluationMessage::Jitter),
            field("Seed", "1", &self.seed, EvaluationMessage::Seed),
        ]
        .spacing(4);

        let chosen = self.scenarios.iter().filter(|s| s.1).count();
        let small = |label: &'static str, message: EvaluationMessage| {
            button(text(label).size(11)).padding([2, 8]).style(btn(Btn::Ghost)).on_press_maybe(enabled.then_some(message))
        };
        let mut list = column![row![
            text(format!("Scenarios ({chosen}/{})", self.scenarios.len())).size(12).color(theme::TEXT_2).width(Length::Fill),
            small("All", EvaluationMessage::AllScenarios(true)),
            small("None", EvaluationMessage::AllScenarios(false)),
            small("Reload", EvaluationMessage::RefreshScenarios),
        ]
        .spacing(4)
        .align_y(Alignment::Center)]
        .spacing(2);
        if self.scenarios.is_empty() {
            list = list.push(text("No scenarios/*.toml in the working directory").size(11).color(theme::MUTED));
        }
        // Two columns of checkboxes.
        let half = self.scenarios.len().div_ceil(2);
        let mut cols = row![].spacing(8);
        for part in [0..half, half..self.scenarios.len()] {
            let mut c = column![].spacing(2).width(Length::FillPortion(1));
            for i in part {
                let (name, on) = &self.scenarios[i];
                let mut cb = checkbox(*on).label(name.clone()).style(theme::check).size(13).text_size(12);
                if enabled {
                    cb = cb.on_toggle(move |v| EvaluationMessage::ToggleScenario(i, v));
                }
                c = c.push(cb);
            }
            cols = cols.push(c);
        }
        list = list.push(cols);

        group(column![section_title("SETUP"), fields, list, text(
            "Candidate = the working tree's Lua. Baseline = the same files at the git revision, run with the working tree's scenarios, harness, simulator.toml and robot_settings.toml. Episode 1 starts as saved; the others are jittered, identically on both sides."
        ).size(10).color(theme::MUTED)].spacing(8).into())
    }

    fn view_run(&self) -> Element<'_, EvaluationMessage> {
        let action = if self.run.is_some() {
            button(text("Cancel").size(12)).padding([4, 14]).style(btn(Btn::Neutral)).on_press(EvaluationMessage::Cancel)
        } else {
            button(text("Run evaluation").size(12)).padding([4, 14]).style(btn(Btn::Play)).on_press(EvaluationMessage::Run)
        };
        let mut line = row![action].spacing(10).align_y(Alignment::Center);
        if self.run.is_some() {
            let p = &self.progress;
            let fraction = if p.total == 0 { 0.0 } else { p.done as f32 / p.total as f32 };
            line = line.push(
                column![
                    progress_bar(0.0..=1.0, fraction).girth(6),
                    text(format!("{} · {}/{} episodes", p.phase, p.done, p.total)).size(11).color(theme::MUTED),
                ]
                .spacing(3)
                .width(Length::Fill),
            );
        } else {
            line = line.push(Space::new().width(Length::Fill));
            line = line.push(
                button(text("Load last result").size(11)).padding([2, 8]).style(btn(Btn::Ghost)).on_press(EvaluationMessage::LoadLatest),
            );
        }
        line.into()
    }
}

impl Shown {
    fn new(evaluation: Evaluation, path: Option<PathBuf>) -> Self {
        let summary = stats::summarize(&evaluation);
        let base_label = format!("Baseline {}", evaluation.baseline.label);
        let cand_label = format!("Candidate ({})", evaluation.candidate.label);

        // Rows: every scenario, then All.
        let rows: Vec<&Comparison> = summary.scenarios.iter().chain(std::iter::once(&summary.overall)).collect();
        let names: Vec<String> = rows.iter().map(|c| c.name.clone()).collect();
        let estimate = |e: stats::Estimate| Some(e.value).filter(|v| v.is_finite());
        let error = |e: stats::Estimate| Some((e.lo, e.hi));

        let win_rates = CategoryChart::new("Win rate (95% Wilson interval)", names.clone())
            .value_axis(Axis::new("episodes with a goal").range(0.0, 1.0).format(TickFormat::Percent))
            .layer(Layer::Bars {
                name: base_label.clone(),
                color: BASELINE,
                values: rows.iter().map(|c| estimate(c.baseline)).collect(),
                errors: rows.iter().map(|c| error(c.baseline)).collect(),
            })
            .layer(Layer::Bars {
                name: cand_label.clone(),
                color: CANDIDATE,
                values: rows.iter().map(|c| estimate(c.candidate)).collect(),
                errors: rows.iter().map(|c| error(c.candidate)).collect(),
            });

        let differences = CategoryChart::new("Candidate − baseline win rate (95% bootstrap interval)", names)
            .value_axis(Axis::new("difference in win rate").with_zero().format(TickFormat::Percent))
            .ref_line(RefLine::new(0.0).label("no change"))
            .layer(Layer::Points {
                name: String::new(),
                color: theme::ACCENT,
                values: rows.iter().map(|c| c.diff.value.is_finite().then(|| Interval::new(c.diff.value, c.diff.lo, c.diff.hi))).collect(),
                emphasis: rows.iter().map(|c| c.name == "All").collect(),
            })
            .without_legend();

        // Outcome shares per side.
        let mut labels: Vec<String> = summary.outcomes_baseline.keys().chain(summary.outcomes_candidate.keys()).cloned().collect();
        labels.sort_by_key(|l| outcome_rank(l));
        labels.dedup();
        let total_b: usize = summary.outcomes_baseline.values().sum();
        let total_c: usize = summary.outcomes_candidate.values().sum();
        let share = |map: &std::collections::BTreeMap<String, usize>, total: usize, l: &str| {
            if total == 0 { 0.0 } else { *map.get(l).unwrap_or(&0) as f64 / total as f64 }
        };
        let outcomes = CategoryChart::new("Episode outcomes", vec!["Baseline".into(), "Candidate".into()])
            .value_axis(Axis::new("share of episodes").range(0.0, 1.0).format(TickFormat::Percent))
            .layer(Layer::Stacked {
                segments: labels
                    .iter()
                    .map(|l| {
                        (l.clone(), outcome_color(l), vec![share(&summary.outcomes_baseline, total_b, l), share(&summary.outcomes_candidate, total_c, l)])
                    })
                    .collect(),
            });

        let times = CategoryChart::new("Time to goal (box: quartiles, whiskers: 1.5·IQR)", vec!["Baseline".into(), "Candidate".into()])
            .value_axis(Axis::new("s from the start").with_zero().format(TickFormat::Seconds))
            .layer(Layer::Boxes { name: String::new(), color: theme::ACCENT, samples: vec![summary.goal_times_baseline.clone(), summary.goal_times_candidate.clone()] })
            .without_legend();

        let t_max = evaluation.settings.episode_time;
        let grid: Vec<f64> = (0..=200).map(|i| t_max * i as f64 / 200.0).collect();
        let curve = XyChart::new(
            "Cumulative goals: share of episodes scored by time t",
            Axis::new("t (s)").range(0.0, t_max),
            Axis::new("scored").range(0.0, 1.0).format(TickFormat::Percent),
        )
        .series(Series::new(base_label, BASELINE, SeriesKind::Step, stats::goal_curve(&summary.goal_times_baseline, evaluation.baseline.episodes.len(), &grid)))
        .series(Series::new(cand_label, CANDIDATE, SeriesKind::Step, stats::goal_curve(&summary.goal_times_candidate, evaluation.candidate.episodes.len(), &grid)));

        Self { evaluation, summary, path, win_rates, differences, outcomes, times, curve }
    }

    fn view(&self) -> Element<'_, EvaluationMessage> {
        let s = &self.summary;
        let o = &s.overall;
        let (word, fg, bg) = match s.verdict {
            Verdict::Better => ("BETTER", theme::GREEN, theme::GREEN_DIM),
            Verdict::Worse => ("WORSE", theme::RED_FG, theme::RED_DIM),
            Verdict::Inconclusive => ("NO SIGNIFICANT DIFFERENCE", theme::ORANGE, theme::ORANGE_DIM),
        };
        let banner = container(
            column![
                row![
                    text(word).size(14).font(theme::SANS_BOLD).color(fg),
                    Space::new().width(Length::Fill),
                    text(format!("n = {} paired episodes", o.n)).size(11).color(theme::MUTED),
                ]
                .align_y(Alignment::Center),
                text(format!(
                    "Win rate {} → {}  (Δ {} pts, 95% CI {} to {})",
                    pct(o.baseline.value),
                    pct(o.candidate.value),
                    signed_pts(o.diff.value),
                    signed_pts(o.diff.lo),
                    signed_pts(o.diff.hi),
                ))
                .size(12)
                .color(theme::TEXT),
                text(format!(
                    "Exact McNemar p = {}  ·  won only by baseline {} · only by candidate {}  ·  α = {}",
                    p_text(o.p),
                    o.only_baseline,
                    o.only_candidate,
                    stats::ALPHA
                ))
                .size(11)
                .color(theme::TEXT_2),
                text(match o.time_diff {
                    Some(t) => format!("Time to goal Δ {:+.2} s (95% CI {:+.2} to {:+.2}) over episodes both sides scored", t.value, t.lo, t.hi),
                    None => "Time to goal: no episode scored by both sides".into(),
                })
                .size(11)
                .color(theme::TEXT_2),
            ]
            .spacing(3),
        )
        .padding([8, 10])
        .width(Length::Fill)
        .style(move |_| container::Style { background: Some(Background::Color(bg)), border: Border { radius: 4.0.into(), ..Default::default() }, ..Default::default() });

        // Table.
        let cell = |t: String, w: f32, c: Color| text(t).size(11).font(theme::MONO).color(c).width(Length::Fixed(w)).align_x(Horizontal::Right);
        let mut table = column![row![
            text("Scenario").size(11).color(theme::MUTED).width(Length::Fill),
            cell("Base".into(), 48.0, theme::MUTED),
            cell("Cand".into(), 48.0, theme::MUTED),
            cell("Δ pts [95% CI]".into(), 132.0, theme::MUTED),
            cell("p Holm".into(), 58.0, theme::MUTED),
        ]
        .spacing(6)]
        .spacing(2);
        for c in s.scenarios.iter().chain(std::iter::once(o)) {
            let p = c.p_adjusted;
            let significant = p < stats::ALPHA;
            let color = if !significant { theme::TEXT_2 } else if c.diff.value > 0.0 { theme::GREEN } else { theme::RED_FG };
            let bold = c.name == "All";
            table = table.push(
                row![
                    text(c.name.clone()).size(11).font(if bold { theme::SANS_BOLD } else { theme::SANS }).color(theme::TEXT).width(Length::Fill),
                    cell(pct(c.baseline.value), 48.0, theme::TEXT_2),
                    cell(pct(c.candidate.value), 48.0, theme::TEXT_2),
                    cell(format!("{} [{}, {}]", signed_pts(c.diff.value), signed_pts(c.diff.lo), signed_pts(c.diff.hi)), 132.0, color),
                    cell(p_text(p), 58.0, color),
                ]
                .spacing(6),
            );
        }

        let width = 600.0;
        let mut info = format!(
            "{} · {} episodes × {} scenarios · {} s · jitter {} m · seed {}",
            self.evaluation.settings.entry,
            self.evaluation.settings.repeats,
            self.evaluation.settings.scenarios.len(),
            self.evaluation.settings.episode_time,
            self.evaluation.settings.jitter,
            self.evaluation.settings.seed
        );
        if s.unpaired > 0 {
            info.push_str(&format!(" · {} unpaired episodes left out", s.unpaired));
        }
        if let Some(path) = &self.path {
            info.push_str(&format!(" · {}", path.display()));
        }

        group(
            column![
                section_title("RESULT"),
                banner,
                table,
                self.win_rates.view(horizontal_height(&self.win_rates, ROW * 1.4, width)),
                self.differences.view(horizontal_height(&self.differences, ROW, width)),
                row![self.outcomes.view(150.0), self.times.view(150.0)].spacing(8),
                self.curve.view(220.0),
                text("p Holm: exact McNemar per scenario, Holm-adjusted for the number of scenarios. The verdict uses the unadjusted test over all episodes.").size(10).color(theme::MUTED),
                text(info).size(10).color(theme::MUTED),
            ]
            .spacing(8)
            .into(),
        )
    }
}

fn pct(v: f64) -> String {
    if v.is_finite() { format!("{:.0}%", v * 100.0) } else { "–".into() }
}

fn signed_pts(v: f64) -> String {
    if v.is_finite() { format!("{:+.0}", v * 100.0) } else { "–".into() }
}

fn p_text(p: f64) -> String {
    if p < 0.001 { "<0.001".into() } else { format!("{p:.3}") }
}

/// Outcomes from best to worst for blue (stack order).
fn outcome_rank(label: &str) -> usize {
    ["goal", "saved", "wide", "out", "cleared", "stolen", "timeout", "foul", "own_area", "conceded"]
        .iter()
        .position(|l| *l == label)
        .unwrap_or(99)
}

fn outcome_color(label: &str) -> Color {
    match label {
        "goal" => PALETTE[2],
        "saved" => PALETTE[4],
        "out" | "wide" => Color { r: 0.45, g: 0.47, b: 0.5, a: 1.0 },
        "cleared" => PALETTE[3],
        "stolen" => PALETTE[0],
        "timeout" => Color { r: 0.3, g: 0.32, b: 0.35, a: 1.0 },
        "foul" => PALETTE[6],
        "own_area" | "conceded" => PALETTE[5],
        _ => PALETTE[7],
    }
}

fn section_title(title: &str) -> Element<'_, EvaluationMessage> {
    text(title).size(11).font(theme::MONO_BOLD).color(theme::MUTED).into()
}

fn group(content: Element<'_, EvaluationMessage>) -> Element<'_, EvaluationMessage> {
    container(content)
        .padding([8, 10])
        .width(Length::Fill)
        .style(theme::group)
        .into()
}
