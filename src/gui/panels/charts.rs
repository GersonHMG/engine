// gui/panels/charts.rs — Customizable robot charts panel
//
// Features:
// - "+" button to add plots dynamically
// - Per-plot X/Y axis variable selection via dropdowns
// - Recording mode: captures all samples for later export
// - CSV export of recorded data
// - Live mode (not recording): rolling window display without storage

use iced::widget::canvas::{self, Frame, Geometry, Path, Stroke, Text};
use iced::widget::{button, column, container, pick_list, row, scrollable, text, Canvas};
use iced::{mouse, Color, Element, Length, Point, Rectangle, Size, Theme};
use std::collections::VecDeque;
use std::fmt;

const LIVE_BUFFER_SIZE: usize = 600;

/// Available variables for plotting axes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlotVariable {
    Time,
    X,
    Y,
    Theta,
    Vx,
    Vy,
    CmdVx,
    CmdVy,
    CmdAngular,
}

impl fmt::Display for PlotVariable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Time => write!(f, "Time"),
            Self::X => write!(f, "X"),
            Self::Y => write!(f, "Y"),
            Self::Theta => write!(f, "θ"),
            Self::Vx => write!(f, "Vx"),
            Self::Vy => write!(f, "Vy"),
            Self::CmdVx => write!(f, "Cmd Vx"),
            Self::CmdVy => write!(f, "Cmd Vy"),
            Self::CmdAngular => write!(f, "Cmd ω"),
        }
    }
}

const ALL_VARIABLES: &[PlotVariable] = &[
    PlotVariable::Time,
    PlotVariable::X,
    PlotVariable::Y,
    PlotVariable::Theta,
    PlotVariable::Vx,
    PlotVariable::Vy,
    PlotVariable::CmdVx,
    PlotVariable::CmdVy,
    PlotVariable::CmdAngular,
];

/// A single snapshot of all robot variables at one instant.
#[derive(Debug, Clone)]
pub struct DataSample {
    pub time: f64,
    pub x: f64,
    pub y: f64,
    pub theta: f64,
    pub vx: f64,
    pub vy: f64,
    pub cmd_vx: f64,
    pub cmd_vy: f64,
    pub cmd_angular: f64,
}

impl DataSample {
    pub fn get(&self, var: PlotVariable) -> f64 {
        match var {
            PlotVariable::Time => self.time,
            PlotVariable::X => self.x,
            PlotVariable::Y => self.y,
            PlotVariable::Theta => self.theta,
            PlotVariable::Vx => self.vx,
            PlotVariable::Vy => self.vy,
            PlotVariable::CmdVx => self.cmd_vx,
            PlotVariable::CmdVy => self.cmd_vy,
            PlotVariable::CmdAngular => self.cmd_angular,
        }
    }
}

/// A single Y-axis series within a plot.
pub struct PlotSeries {
    pub var: PlotVariable,
    pub color: Color,
    pub live_buffer: VecDeque<(f64, f64)>,
}

/// A single user-created plot with one X variable and multiple Y series.
pub struct Plot {
    pub x_var: PlotVariable,
    pub series: Vec<PlotSeries>,
}

impl Plot {
    pub fn new(x_var: PlotVariable, y_var: PlotVariable, color: Color) -> Self {
        Self {
            x_var,
            series: vec![PlotSeries {
                var: y_var,
                color,
                live_buffer: VecDeque::with_capacity(LIVE_BUFFER_SIZE),
            }],
        }
    }

    pub fn push_live(&mut self, sample: &DataSample) {
        let x = sample.get(self.x_var);
        for s in &mut self.series {
            let y = sample.get(s.var);
            if s.live_buffer.len() >= LIVE_BUFFER_SIZE {
                s.live_buffer.pop_front();
            }
            s.live_buffer.push_back((x, y));
        }
    }

    pub fn clear_all_live(&mut self) {
        for s in &mut self.series {
            s.live_buffer.clear();
        }
    }
}

const PLOT_COLORS: &[Color] = &[
    Color { r: 1.0, g: 0.27, b: 0.27, a: 1.0 },
    Color { r: 0.27, g: 1.0, b: 0.27, a: 1.0 },
    Color { r: 0.27, g: 0.53, b: 1.0, a: 1.0 },
    Color { r: 1.0, g: 0.53, b: 0.27, a: 1.0 },
    Color { r: 0.27, g: 0.87, b: 1.0, a: 1.0 },
    Color { r: 0.87, g: 0.53, b: 1.0, a: 1.0 },
    Color { r: 1.0, g: 1.0, b: 0.27, a: 1.0 },
    Color { r: 1.0, g: 0.27, b: 0.87, a: 1.0 },
];

// ─── Messages ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum ChartsMessage {
    AddPlot,
    RemovePlot(usize),
    SetXVar(usize, PlotVariable),
    /// Set the Y variable for series `series_idx` inside plot `plot_idx`.
    SetYVar(usize, usize, PlotVariable),
    /// Add a new Y series to plot `plot_idx`.
    AddYVar(usize),
    /// Remove Y series `series_idx` from plot `plot_idx`.
    RemoveYVar(usize, usize),
    ToggleRecording,
    ExportCsv,
    ExportComplete,
}

// ─── Panel state ─────────────────────────────────────────────────────────────

pub struct ChartsPanel {
    pub plots: Vec<Plot>,
    pub recording: bool,
    pub recorded_samples: Vec<DataSample>,
    pub start_time: std::time::Instant,
    pub active: bool,
    next_color_index: usize,
}

impl ChartsPanel {
    pub fn new() -> Self {
        Self {
            plots: Vec::new(),
            recording: false,
            recorded_samples: Vec::new(),
            start_time: std::time::Instant::now(),
            active: false,
            next_color_index: 0,
        }
    }

    fn next_color(&mut self) -> Color {
        let color = PLOT_COLORS[self.next_color_index % PLOT_COLORS.len()];
        self.next_color_index += 1;
        color
    }

    /// Feed a new data sample. Called every vision tick.
    pub fn push_sample(&mut self, sample: DataSample) {
        if self.recording {
            self.recorded_samples.push(sample.clone());
        }
        // Always update live buffers so the rolling window works
        for plot in &mut self.plots {
            plot.push_live(&sample);
        }
    }

    /// Handle messages that don't require iced::Task.
    /// Returns `true` for ExportCsv (caller must spawn async task).
    pub fn update(&mut self, msg: &ChartsMessage) -> bool {
        match msg {
            ChartsMessage::AddPlot => {
                let color = self.next_color();
                self.plots.push(Plot::new(PlotVariable::Time, PlotVariable::Vx, color));
                false
            }
            ChartsMessage::RemovePlot(idx) => {
                if *idx < self.plots.len() {
                    self.plots.remove(*idx);
                }
                false
            }
            ChartsMessage::SetXVar(idx, var) => {
                if let Some(plot) = self.plots.get_mut(*idx) {
                    plot.x_var = *var;
                    plot.clear_all_live();
                }
                false
            }
            ChartsMessage::SetYVar(plot_idx, series_idx, var) => {
                if let Some(plot) = self.plots.get_mut(*plot_idx) {
                    if let Some(series) = plot.series.get_mut(*series_idx) {
                        series.var = *var;
                        series.live_buffer.clear();
                    }
                }
                false
            }
            ChartsMessage::AddYVar(plot_idx) => {
                if let Some(plot) = self.plots.get_mut(*plot_idx) {
                    let color = PLOT_COLORS[self.next_color_index % PLOT_COLORS.len()];
                    self.next_color_index += 1;
                    plot.series.push(PlotSeries {
                        var: PlotVariable::Vy,
                        color,
                        live_buffer: VecDeque::with_capacity(LIVE_BUFFER_SIZE),
                    });
                }
                false
            }
            ChartsMessage::RemoveYVar(plot_idx, series_idx) => {
                if let Some(plot) = self.plots.get_mut(*plot_idx) {
                    if plot.series.len() > 1 && *series_idx < plot.series.len() {
                        plot.series.remove(*series_idx);
                    }
                }
                false
            }
            ChartsMessage::ToggleRecording => {
                if self.recording {
                    // Stop recording — keep recorded_samples for export
                    self.recording = false;
                } else {
                    // Start recording — clear old data
                    self.recorded_samples.clear();
                    self.start_time = std::time::Instant::now();
                    self.recording = true;
                }
                false
            }
            ChartsMessage::ExportCsv => true, // handled externally with async task
            ChartsMessage::ExportComplete => false,
        }
    }

    /// Clone recorded samples for the async CSV export task.
    pub fn recorded_samples_clone(&self) -> Vec<DataSample> {
        self.recorded_samples.clone()
    }

    pub fn view(&self) -> Element<'_, ChartsMessage> {
        // ── Header row ───────────────────────────────────────────────────
        let add_btn = button(text("+").size(16))
            .on_press(ChartsMessage::AddPlot)
            .style(button::success)
            .width(Length::Fixed(32.0))
            .height(Length::Fixed(28.0));

        let record_btn = if self.recording {
            button(text("⏹ Stop").size(11))
                .on_press(ChartsMessage::ToggleRecording)
                .style(button::danger)
        } else {
            button(text("⏺ Record").size(11))
                .on_press(ChartsMessage::ToggleRecording)
                .style(button::secondary)
        };

        let can_export = !self.recording && !self.recorded_samples.is_empty();
        let export_btn = button(text("📁 Export CSV").size(11))
            .on_press_maybe(if can_export { Some(ChartsMessage::ExportCsv) } else { None })
            .style(button::secondary);

        let status: Element<ChartsMessage> = if self.recording {
            text(format!("⏺ Recording ({} samples)", self.recorded_samples.len()))
                .size(10)
                .color(Color::from_rgb(1.0, 0.3, 0.3))
                .into()
        } else if !self.recorded_samples.is_empty() {
            text(format!("Recorded {} samples", self.recorded_samples.len()))
                .size(10)
                .color(Color::from_rgb(0.5, 0.5, 0.5))
                .into()
        } else {
            text("Live")
                .size(10)
                .color(Color::from_rgb(0.4, 0.8, 0.4))
                .into()
        };

        let header = row![add_btn, record_btn, export_btn, status]
            .spacing(8)
            .padding([8, 12])
            .align_y(iced::Alignment::Center);

        // ── Plot cards ───────────────────────────────────────────────────
        let mut plots_col = column![].spacing(8).padding([12, 12]);

        if self.plots.is_empty() {
            plots_col = plots_col.push(
                container(
                    text("Click + to add a plot")
                        .size(12)
                        .color(Color::from_rgb(0.5, 0.5, 0.5)),
                )
                .padding(20)
                .center_x(Length::Fill),
            );
        }

        for (i, plot) in self.plots.iter().enumerate() {
            let x_pick = pick_list(
                ALL_VARIABLES.to_vec(),
                Some(plot.x_var),
                move |var| ChartsMessage::SetXVar(i, var),
            )
            .text_size(10)
            .width(Length::Fixed(90.0));

            let remove_btn = button(text("✕").size(10))
                .on_press(ChartsMessage::RemovePlot(i))
                .style(button::danger)
                .padding([2, 6]);

            let add_y_btn = button(text("+Y").size(9))
                .on_press(ChartsMessage::AddYVar(i))
                .style(button::success)
                .padding([2, 6]);

            // X axis row + plot-level controls
            let x_row = row![
                text("X:").size(10),
                x_pick,
                add_y_btn,
                iced::widget::Space::new().width(Length::Fill),
                remove_btn,
            ]
            .spacing(4)
            .align_y(iced::Alignment::Center);

            // Y-series rows
            let mut series_col = column![].spacing(2);
            let mut all_series_data: Vec<(Vec<(f64, f64)>, Color, String)> = Vec::new();

            for (j, series) in plot.series.iter().enumerate() {
                let y_pick = pick_list(
                    ALL_VARIABLES.to_vec(),
                    Some(series.var),
                    move |var| ChartsMessage::SetYVar(i, j, var),
                )
                .text_size(10)
                .width(Length::Fixed(90.0));

                let color_dot = container(
                    text("●").size(10).color(series.color),
                )
                .padding([0, 2]);

                let can_remove = plot.series.len() > 1;
                let remove_y_btn = button(text("✕").size(8))
                    .on_press_maybe(if can_remove { Some(ChartsMessage::RemoveYVar(i, j)) } else { None })
                    .style(button::secondary)
                    .padding([1, 4]);

                let y_row = row![color_dot, text("Y:").size(10), y_pick, remove_y_btn]
                    .spacing(4)
                    .align_y(iced::Alignment::Center);
                series_col = series_col.push(y_row);

                // Collect data for this series
                let data: Vec<(f64, f64)> = if self.recording {
                    self.recorded_samples
                        .iter()
                        .map(|s| (s.get(plot.x_var), s.get(series.var)))
                        .collect()
                } else {
                    series.live_buffer.iter().copied().collect()
                };
                all_series_data.push((data, series.color, series.var.to_string()));
            }

            let y_labels: String = all_series_data.iter().map(|(_, _, l)| l.as_str()).collect::<Vec<_>>().join(", ");

            let chart = Canvas::new(MultiSeriesChartProgram {
                series: all_series_data,
                x_label: plot.x_var.to_string(),
                y_label: y_labels,
            })
            .width(Length::Fill)
            .height(Length::Fixed(150.0));

            let plot_card = container(column![x_row, series_col, chart].spacing(4))
                .padding(8)
                .style(plot_card_style);

            plots_col = plots_col.push(plot_card);
        }

        let content = column![header, scrollable(plots_col).height(Length::Fill)];

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(panel_style)
            .into()
    }
}

// ─── CSV export (runs inside an async task) ──────────────────────────────────

pub async fn export_csv_async(samples: Vec<DataSample>) {
    let path = rfd::AsyncFileDialog::new()
        .add_filter("CSV", &["csv"])
        .set_file_name("chart_data.csv")
        .save_file()
        .await;

    if let Some(handle) = path {
        let path = handle.path().to_path_buf();
        // Write on a blocking thread to avoid blocking the async runtime
        let _ = tokio::task::spawn_blocking(move || {
            if let Ok(mut wtr) = csv::Writer::from_path(&path) {
                let _ = wtr.write_record([
                    "time", "x", "y", "theta", "vx", "vy", "cmd_vx", "cmd_vy", "cmd_angular",
                ]);
                for s in &samples {
                    let _ = wtr.write_record(&[
                        s.time.to_string(),
                        s.x.to_string(),
                        s.y.to_string(),
                        s.theta.to_string(),
                        s.vx.to_string(),
                        s.vy.to_string(),
                        s.cmd_vx.to_string(),
                        s.cmd_vy.to_string(),
                        s.cmd_angular.to_string(),
                    ]);
                }
                let _ = wtr.flush();
            }
        })
        .await;
    }
}

// ─── Canvas program for multi-series XY line chart ──────────────────────────

struct MultiSeriesChartProgram {
    /// Each entry: (data points, color, label)
    series: Vec<(Vec<(f64, f64)>, Color, String)>,
    x_label: String,
    y_label: String,
}

impl canvas::Program<ChartsMessage> for MultiSeriesChartProgram {
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
        let w = bounds.width;
        let h = bounds.height;

        // Background
        frame.fill_rectangle(
            Point::ORIGIN,
            Size::new(w, h),
            Color::from_rgb(0.1, 0.1, 0.18),
        );

        let has_data = self.series.iter().any(|(d, _, _)| d.len() >= 2);
        if !has_data {
            frame.fill_text(Text {
                content: "No data".to_string(),
                position: Point::new(w / 2.0, h / 2.0),
                color: Color::from_rgb(0.4, 0.4, 0.4),
                size: iced::Pixels(11.0),
                align_x: iced::alignment::Horizontal::Center.into(),
                align_y: iced::alignment::Vertical::Center.into(),
                ..Text::default()
            });
            return vec![frame.into_geometry()];
        }

        // Margins
        let ml = 50.0f32;
        let mr = 10.0f32;
        let mt = 10.0f32;
        let mb = 20.0f32;
        let pw = w - ml - mr;
        let ph = h - mt - mb;

        // Compute global extent across all series
        let (mut x_min, mut x_max, mut y_min, mut y_max) = (
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
        );
        for (data, _, _) in &self.series {
            for &(x, y) in data {
                if x < x_min { x_min = x; }
                if x > x_max { x_max = x; }
                if y < y_min { y_min = y; }
                if y > y_max { y_max = y; }
            }
        }
        let x_range = x_max - x_min;
        let y_range = y_max - y_min;
        let x_pad = if x_range < 0.01 { 1.0 } else { x_range * 0.05 };
        let y_pad = if y_range < 0.01 { 1.0 } else { y_range * 0.1 };
        x_min -= x_pad;
        x_max += x_pad;
        y_min -= y_pad;
        y_max += y_pad;
        let x_span = x_max - x_min;
        let y_span = y_max - y_min;

        // Grid lines (horizontal)
        let grid_color = Color::from_rgb(0.2, 0.2, 0.25);
        let label_color = Color::from_rgb(0.5, 0.5, 0.5);
        for i in 0..=4 {
            let frac = i as f32 / 4.0;
            let py = mt + ph * (1.0 - frac);
            let line = Path::line(Point::new(ml, py), Point::new(ml + pw, py));
            frame.stroke(
                &line,
                Stroke::default().with_color(grid_color).with_width(0.5),
            );
            let val = y_min + y_span * frac as f64;
            frame.fill_text(Text {
                content: format!("{:.2}", val),
                position: Point::new(ml - 4.0, py),
                color: label_color,
                size: iced::Pixels(8.0),
                align_x: iced::alignment::Horizontal::Right.into(),
                align_y: iced::alignment::Vertical::Center.into(),
                ..Text::default()
            });
        }

        // Draw each series line
        if x_span > 0.0 && y_span > 0.0 {
            for (data, color, _) in &self.series {
                if data.len() < 2 {
                    continue;
                }
                let path = Path::new(|b| {
                    for (i, &(x, y)) in data.iter().enumerate() {
                        let px = ml + ((x - x_min) / x_span) as f32 * pw;
                        let py = mt + ph - ((y - y_min) / y_span) as f32 * ph;
                        if i == 0 {
                            b.move_to(Point::new(px, py));
                        } else {
                            b.line_to(Point::new(px, py));
                        }
                    }
                });
                frame.stroke(
                    &path,
                    Stroke::default().with_color(*color).with_width(1.5),
                );
            }
        }

        // Axis labels
        frame.fill_text(Text {
            content: self.x_label.to_string(),
            position: Point::new(ml + pw / 2.0, h - 2.0),
            color: label_color,
            size: iced::Pixels(9.0),
            align_x: iced::alignment::Horizontal::Center.into(),
            align_y: iced::alignment::Vertical::Bottom.into(),
            ..Text::default()
        });
        frame.fill_text(Text {
            content: self.y_label.to_string(),
            position: Point::new(3.0, mt),
            color: label_color,
            size: iced::Pixels(9.0),
            align_x: iced::alignment::Horizontal::Left.into(),
            align_y: iced::alignment::Vertical::Top.into(),
            ..Text::default()
        });

        // Last value badges — one per series, stacked vertically
        for (idx, (data, color, _)) in self.series.iter().enumerate() {
            if let Some(&(_, last_y)) = data.last() {
                frame.fill_text(Text {
                    content: format!("{:.2}", last_y),
                    position: Point::new(w - mr - 3.0, mt + 2.0 + idx as f32 * 12.0),
                    color: *color,
                    size: iced::Pixels(9.0),
                    align_x: iced::alignment::Horizontal::Right.into(),
                    align_y: iced::alignment::Vertical::Top.into(),
                    ..Text::default()
                });
            }
        }

        vec![frame.into_geometry()]
    }
}

// ─── Styles ──────────────────────────────────────────────────────────────────

fn panel_style(theme: &iced::Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(iced::Background::Color(palette.background.weak.color)),
        border: iced::Border {
            color: palette.background.strong.color,
            width: 1.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

fn plot_card_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(Color::from_rgb(0.12, 0.12, 0.16))),
        border: iced::Border {
            color: Color::from_rgb(0.25, 0.25, 0.3),
            width: 1.0,
            radius: 4.0.into(),
        },
        ..Default::default()
    }
}
