// gui/plot — A small scientific plotting library on iced's canvas
//
// Two chart kinds, each a plain struct you fill and draw with `.view(height)`:
//   * `CategoryChart`: one value per category (scenario, side, …), horizontal or vertical:
//       - `Layer::Bars`    grouped bars, optional error bars (confidence intervals);
//       - `Layer::Boxes`   box plots from raw samples (median, quartiles, 1.5·IQR whiskers,
//                          outliers);
//       - `Layer::Points`  point estimates with interval whiskers (a forest plot);
//       - `Layer::Stacked` stacked bars (a composition: outcome shares).
//     Layers other than Stacked are dodged side by side within each category.
//   * `XyChart`: numeric x and y, with `Series` drawn as lines, steps or scatter points,
//     optionally with a shaded band (an interval around a curve).
// Both have nice-number axes (`scale`), reference lines (zero, chance level, …) and a legend.
// Colors default to the engine theme (`Style::default`); `PALETTE` is a colorblind-safe set
// (Okabe–Ito) for series.

// A library: the engine uses only part of its API so far (vertical charts, lines, scatter,
// bands and reference-line styling are there for the next panels).
#![allow(dead_code)]

pub mod chart;
pub mod scale;
pub mod xy;

pub use scale::TickFormat;

use iced::widget::canvas::{Frame, LineDash, Path, Stroke, Text};
use iced::{alignment, Color, Pixels, Point, Size};

use crate::gui::theme;

/// Okabe–Ito: distinguishable with every common color-vision deficiency.
pub const PALETTE: [Color; 8] = [
    Color { r: 0.902, g: 0.624, b: 0.0, a: 1.0 },   // orange
    Color { r: 0.337, g: 0.706, b: 0.914, a: 1.0 }, // sky blue
    Color { r: 0.0, g: 0.620, b: 0.451, a: 1.0 },   // bluish green
    Color { r: 0.941, g: 0.894, b: 0.259, a: 1.0 }, // yellow
    Color { r: 0.0, g: 0.447, b: 0.698, a: 1.0 },   // blue
    Color { r: 0.835, g: 0.369, b: 0.0, a: 1.0 },   // vermillion
    Color { r: 0.800, g: 0.475, b: 0.655, a: 1.0 }, // reddish purple
    Color { r: 0.6, g: 0.6, b: 0.6, a: 1.0 },       // grey
];

/// Colors and text sizes of a chart.
#[derive(Debug, Clone, Copy)]
pub struct Style {
    pub background: Color,
    pub grid: Color,
    pub axis: Color,
    pub text: Color,
    pub muted: Color,
    pub title_size: f32,
    pub label_size: f32,
    pub tick_size: f32,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            background: theme::FIELD_BG,
            grid: Color { a: 0.6, ..theme::DIVIDER },
            axis: theme::BTN_BORDER,
            text: theme::TEXT_2,
            muted: theme::MUTED,
            title_size: 12.0,
            label_size: 11.0,
            tick_size: 10.0,
        }
    }
}

/// A value axis.
#[derive(Debug, Clone, Default)]
pub struct Axis {
    pub label: String,
    /// Fixed ends; None = from the data (widened to nice ticks).
    pub min: Option<f64>,
    pub max: Option<f64>,
    /// Keep 0 inside the axis (bars, differences).
    pub include_zero: bool,
    pub format: TickFormat,
}

impl Axis {
    pub fn new(label: impl Into<String>) -> Self {
        Self { label: label.into(), ..Self::default() }
    }

    pub fn range(mut self, min: f64, max: f64) -> Self {
        self.min = Some(min);
        self.max = Some(max);
        self
    }

    pub fn with_zero(mut self) -> Self {
        self.include_zero = true;
        self
    }

    pub fn format(mut self, format: TickFormat) -> Self {
        self.format = format;
        self
    }

    /// The ticks for data spanning [lo, hi] (None: no data).
    pub(crate) fn ticks(&self, data: Option<(f64, f64)>, target: usize) -> scale::Ticks {
        let (mut lo, mut hi) = data.unwrap_or((0.0, 1.0));
        if self.include_zero {
            lo = lo.min(0.0);
            hi = hi.max(0.0);
        }
        if let Some(min) = self.min {
            lo = min;
        }
        if let Some(max) = self.max {
            hi = max;
        }
        let mut ticks = scale::nice_ticks(lo, hi, target);
        // Fixed ends stay exactly where asked.
        if let (Some(min), Some(max)) = (self.min, self.max) {
            ticks.min = min;
            ticks.max = max;
            ticks.values.retain(|v| *v >= min - 1e-9 && *v <= max + 1e-9);
        }
        ticks
    }
}

/// An estimate with its interval.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Interval {
    pub value: f64,
    pub lo: f64,
    pub hi: f64,
}

impl Interval {
    pub fn new(value: f64, lo: f64, hi: f64) -> Self {
        Self { value, lo, hi }
    }
}

/// A horizontal (XY: y = value; categories: along the value axis) or vertical line.
#[derive(Debug, Clone)]
pub struct RefLine {
    pub value: f64,
    pub color: Color,
    pub dashed: bool,
    pub label: Option<String>,
}

impl RefLine {
    pub fn new(value: f64) -> Self {
        Self { value, color: theme::MUTED, dashed: true, label: None }
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn solid(mut self) -> Self {
        self.dashed = false;
        self
    }
}

/// Tukey box-plot statistics of a sample.
#[derive(Debug, Clone, PartialEq)]
pub struct BoxStats {
    pub q1: f64,
    pub median: f64,
    pub q3: f64,
    /// Most extreme points within 1.5·IQR of the box.
    pub whisker_lo: f64,
    pub whisker_hi: f64,
    pub outliers: Vec<f64>,
    pub n: usize,
}

impl BoxStats {
    pub fn from_samples(samples: &[f64]) -> Option<Self> {
        let mut v: Vec<f64> = samples.iter().copied().filter(|x| x.is_finite()).collect();
        if v.is_empty() {
            return None;
        }
        v.sort_by(f64::total_cmp);
        let q = |p: f64| {
            let pos = p * (v.len() - 1) as f64;
            let (i, f) = (pos.floor() as usize, pos.fract());
            if i + 1 < v.len() { v[i] * (1.0 - f) + v[i + 1] * f } else { v[i] }
        };
        let (q1, median, q3) = (q(0.25), q(0.5), q(0.75));
        let iqr = q3 - q1;
        let (fence_lo, fence_hi) = (q1 - 1.5 * iqr, q3 + 1.5 * iqr);
        let inside: Vec<f64> = v.iter().copied().filter(|x| *x >= fence_lo && *x <= fence_hi).collect();
        Some(Self {
            q1,
            median,
            q3,
            whisker_lo: inside.first().copied().unwrap_or(q1),
            whisker_hi: inside.last().copied().unwrap_or(q3),
            outliers: v.iter().copied().filter(|x| *x < fence_lo || *x > fence_hi).collect(),
            n: v.len(),
        })
    }
}

// --- drawing helpers shared by the chart kinds ---

/// Rough width of `text` at `size` px (proportional font): for layout only.
pub(crate) fn text_width(text: &str, size: f32) -> f32 {
    text.chars().count() as f32 * size * 0.58
}

/// `text` cut to fit `width` px, with an ellipsis.
pub(crate) fn fit(text: &str, size: f32, width: f32) -> String {
    let max = (width / (size * 0.58)).floor().max(1.0) as usize;
    if text.chars().count() <= max {
        text.to_string()
    } else {
        let mut cut: String = text.chars().take(max.saturating_sub(1)).collect();
        cut.push('…');
        cut
    }
}

pub(crate) fn label(
    frame: &mut Frame,
    content: impl Into<String>,
    at: Point,
    size: f32,
    color: Color,
    h: alignment::Horizontal,
    v: alignment::Vertical,
) {
    frame.fill_text(Text {
        content: content.into(),
        position: at,
        color,
        size: Pixels(size),
        font: theme::SANS,
        align_x: h.into(),
        align_y: v.into(),
        ..Text::default()
    });
}

pub(crate) fn line(frame: &mut Frame, a: Point, b: Point, color: Color, width: f32, dashed: bool) {
    let stroke = Stroke::default().with_color(color).with_width(width);
    let stroke = if dashed { Stroke { line_dash: LineDash { segments: &[4.0, 3.0], offset: 0 }, ..stroke } } else { stroke };
    frame.stroke(&Path::line(a, b), stroke);
}

/// The legend, left to right from `at` (wrapping at `width`); returns its height.
pub(crate) fn legend(frame: &mut Frame, entries: &[(String, Color)], at: Point, width: f32, style: &Style) -> f32 {
    if entries.is_empty() {
        return 0.0;
    }
    let row_h = style.label_size + 6.0;
    let (mut x, mut y) = (at.x, at.y);
    for (name, color) in entries {
        let w = 12.0 + text_width(name, style.label_size) + 14.0;
        if x > at.x && x + w > at.x + width {
            x = at.x;
            y += row_h;
        }
        frame.fill_rectangle(Point::new(x, y + row_h / 2.0 - 4.0), Size::new(8.0, 8.0), *color);
        label(frame, name.clone(), Point::new(x + 12.0, y + row_h / 2.0), style.label_size, style.text, alignment::Horizontal::Left, alignment::Vertical::Center);
        x += w;
    }
    y + row_h - at.y
}

/// The legend's height when drawn `width` px wide (for layout before drawing).
pub(crate) fn legend_height(entries: &[(String, Color)], width: f32, style: &Style) -> f32 {
    if entries.is_empty() {
        return 0.0;
    }
    let row_h = style.label_size + 6.0;
    let (mut x, mut rows) = (0.0, 1.0);
    for (name, _) in entries {
        let w = 12.0 + text_width(name, style.label_size) + 14.0;
        if x > 0.0 && x + w > width {
            x = 0.0;
            rows += 1.0;
        }
        x += w;
    }
    rows * row_h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn box_stats_follow_tukey() {
        let b = BoxStats::from_samples(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 100.0]).unwrap();
        assert_eq!(b.median, 5.5);
        assert_eq!(b.q1, 3.25);
        assert_eq!(b.q3, 7.75);
        assert_eq!(b.whisker_lo, 1.0);
        assert_eq!(b.whisker_hi, 9.0);
        assert_eq!(b.outliers, vec![100.0]);
        assert_eq!(b.n, 10);
        assert!(BoxStats::from_samples(&[]).is_none());
        let one = BoxStats::from_samples(&[2.0]).unwrap();
        assert_eq!((one.q1, one.median, one.q3), (2.0, 2.0, 2.0));
    }

    #[test]
    fn fixed_axis_ends_are_kept() {
        let t = Axis::new("p").range(0.0, 1.0).ticks(Some((0.2, 0.7)), 5);
        assert_eq!((t.min, t.max), (0.0, 1.0));
        let t = Axis::new("d").with_zero().ticks(Some((0.3, 0.8)), 5);
        assert_eq!(t.min, 0.0);
    }

    #[test]
    fn fit_shortens_long_labels() {
        assert_eq!(fit("short", 10.0, 100.0), "short");
        let cut = fit("a_very_long_scenario_name", 10.0, 50.0);
        assert!(cut.ends_with('…') && cut.chars().count() <= 9, "{cut}");
    }
}
