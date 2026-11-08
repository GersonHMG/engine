// gui/plot/chart.rs — Charts with one value per category: bars, box plots, forest plots,
// stacked bars (see gui/plot/mod.rs)

use iced::alignment::{Horizontal, Vertical};
use iced::widget::canvas::{self, Frame, Geometry, Path, Stroke};
use iced::widget::Canvas;
use iced::{mouse, Color, Element, Length, Point, Rectangle, Renderer, Size, Theme};

use super::scale::{format_tick, Linear};
use super::{fit, label, legend, legend_height, line, text_width, Axis, BoxStats, Interval, RefLine, Style};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Orientation {
    /// Categories along x, values up.
    Vertical,
    /// Categories down the y axis, values along x (long category names read best).
    #[default]
    Horizontal,
}

#[derive(Debug, Clone)]
pub enum Layer {
    /// One bar per category; `errors[i]` = (lo, hi) whiskers.
    Bars { name: String, color: Color, values: Vec<Option<f64>>, errors: Vec<Option<(f64, f64)>> },
    /// One box per category from its raw samples.
    Boxes { name: String, color: Color, samples: Vec<Vec<f64>> },
    /// A point with its interval per category; `emphasis[i]` draws it larger (a summary row).
    Points { name: String, color: Color, values: Vec<Option<Interval>>, emphasis: Vec<bool> },
    /// Segments stacked per category: (name, color, value per category).
    Stacked { segments: Vec<(String, Color, Vec<f64>)> },
}

impl Layer {
    fn legend_entries(&self) -> Vec<(String, Color)> {
        match self {
            Layer::Bars { name, color, .. } | Layer::Boxes { name, color, .. } | Layer::Points { name, color, .. } => {
                if name.is_empty() { Vec::new() } else { vec![(name.clone(), *color)] }
            }
            Layer::Stacked { segments } => segments.iter().map(|(n, c, _)| (n.clone(), *c)).collect(),
        }
    }

    /// Smallest and largest value the layer draws.
    fn extent(&self) -> Option<(f64, f64)> {
        let mut values: Vec<f64> = Vec::new();
        match self {
            Layer::Bars { values: v, errors, .. } => {
                values.extend(v.iter().flatten());
                values.extend(errors.iter().flatten().flat_map(|(lo, hi)| [*lo, *hi]));
            }
            Layer::Boxes { samples, .. } => {
                values.extend(samples.iter().flatten());
            }
            Layer::Points { values: v, .. } => {
                values.extend(v.iter().flatten().flat_map(|i| [i.lo, i.hi, i.value]));
            }
            Layer::Stacked { segments } => {
                let n = segments.iter().map(|(_, _, v)| v.len()).max().unwrap_or(0);
                for i in 0..n {
                    values.push(segments.iter().map(|(_, _, v)| v.get(i).copied().unwrap_or(0.0)).sum());
                }
                values.push(0.0);
            }
        }
        let values: Vec<f64> = values.into_iter().filter(|v| v.is_finite()).collect();
        let lo = values.iter().copied().reduce(f64::min)?;
        let hi = values.iter().copied().reduce(f64::max)?;
        Some((lo, hi))
    }
}

#[derive(Debug, Clone, Default)]
pub struct CategoryChart {
    pub title: String,
    pub categories: Vec<String>,
    pub orientation: Orientation,
    pub value_axis: Axis,
    pub layers: Vec<Layer>,
    pub ref_lines: Vec<RefLine>,
    pub legend: bool,
    /// Text drawn instead of an empty chart.
    pub empty_text: String,
    pub style: Style,
}

impl CategoryChart {
    pub fn new(title: impl Into<String>, categories: Vec<String>) -> Self {
        Self { title: title.into(), categories, legend: true, empty_text: "No data".into(), ..Self::default() }
    }

    pub fn orientation(mut self, orientation: Orientation) -> Self {
        self.orientation = orientation;
        self
    }

    pub fn value_axis(mut self, axis: Axis) -> Self {
        self.value_axis = axis;
        self
    }

    pub fn layer(mut self, layer: Layer) -> Self {
        self.layers.push(layer);
        self
    }

    pub fn ref_line(mut self, line: RefLine) -> Self {
        self.ref_lines.push(line);
        self
    }

    pub fn without_legend(mut self) -> Self {
        self.legend = false;
        self
    }

    /// A canvas `height` px tall drawing the chart, as wide as its container.
    pub fn view<'a, M: 'a>(&'a self, height: f32) -> Element<'a, M> {
        Canvas::new(self).width(Length::Fill).height(Length::Fixed(height)).into()
    }

    fn legend_entries(&self) -> Vec<(String, Color)> {
        if !self.legend {
            return Vec::new();
        }
        let mut entries: Vec<(String, Color)> = Vec::new();
        for entry in self.layers.iter().flat_map(Layer::legend_entries) {
            if !entries.iter().any(|(n, _)| *n == entry.0) {
                entries.push(entry);
            }
        }
        entries
    }

    fn draw_chart(&self, frame: &mut Frame, size: Size) {
        let s = &self.style;
        frame.fill_rectangle(Point::ORIGIN, size, s.background);
        let pad = 8.0;
        let mut top = pad;
        if !self.title.is_empty() {
            label(frame, self.title.clone(), Point::new(pad, top), s.title_size, s.text, Horizontal::Left, Vertical::Top);
            top += s.title_size + 6.0;
        }
        let entries = self.legend_entries();
        if !entries.is_empty() {
            top += legend(frame, &entries, Point::new(pad, top), size.width - 2.0 * pad, s) + 2.0;
        }

        let data = self.layers.iter().filter_map(Layer::extent).reduce(|a, b| (a.0.min(b.0), a.1.max(b.1)));
        let data = match (data, self.ref_lines.iter().map(|r| r.value).reduce(f64::min)) {
            (Some((lo, hi)), Some(_)) => {
                let r_lo = self.ref_lines.iter().map(|r| r.value).fold(lo, f64::min);
                let r_hi = self.ref_lines.iter().map(|r| r.value).fold(hi, f64::max);
                Some((r_lo, r_hi))
            }
            (d, _) => d,
        };
        if self.categories.is_empty() || data.is_none() {
            label(frame, self.empty_text.clone(), Point::new(size.width / 2.0, (top + size.height) / 2.0), s.label_size, s.muted, Horizontal::Center, Vertical::Center);
            return;
        }

        let horizontal = self.orientation == Orientation::Horizontal;
        let value_len = if horizontal { size.width } else { size.height };
        let ticks = self.value_axis.ticks(data, ((value_len / 70.0) as usize).clamp(2, 8));
        let tick_labels: Vec<String> = ticks.values.iter().map(|v| format_tick(*v, ticks.step, self.value_axis.format)).collect();
        let widest_tick = tick_labels.iter().map(|t| text_width(t, s.tick_size)).fold(0.0, f32::max);
        let axis_label_h = if self.value_axis.label.is_empty() { 0.0 } else { s.label_size + 6.0 };

        // Plot area.
        let plot = if horizontal {
            let widest_cat = self.categories.iter().map(|c| text_width(c, s.label_size)).fold(0.0, f32::max);
            let left = pad + widest_cat.min(size.width * 0.38) + 8.0;
            let bottom = size.height - pad - axis_label_h - s.tick_size - 6.0;
            Rectangle::new(Point::new(left, top + 4.0), Size::new((size.width - pad - widest_tick / 2.0 - left).max(10.0), (bottom - top - 4.0).max(10.0)))
        } else {
            let left = pad + widest_tick + 6.0;
            let bottom = size.height - pad - s.label_size - 6.0;
            let top = top + axis_label_h;
            Rectangle::new(Point::new(left, top + 4.0), Size::new((size.width - pad - left).max(10.0), (bottom - top - 4.0).max(10.0)))
        };

        let value = if horizontal {
            Linear::new((ticks.min, ticks.max), (plot.x, plot.x + plot.width))
        } else {
            Linear::new((ticks.min, ticks.max), (plot.y + plot.height, plot.y))
        };
        let n = self.categories.len();
        let band = if horizontal { plot.height } else { plot.width } / n as f32;
        let band_start = |i: usize| if horizontal { plot.y } else { plot.x } + band * i as f32;
        // A point at (value v, category position c) in pixels.
        let at = |v: f64, c: f32| if horizontal { Point::new(value.map(v), c) } else { Point::new(c, value.map(v)) };

        // Grid and ticks.
        for (v, text) in ticks.values.iter().zip(&tick_labels) {
            let p = value.map(*v);
            if horizontal {
                line(frame, Point::new(p, plot.y), Point::new(p, plot.y + plot.height), s.grid, 1.0, false);
                label(frame, text.clone(), Point::new(p, plot.y + plot.height + 4.0), s.tick_size, s.muted, Horizontal::Center, Vertical::Top);
            } else {
                line(frame, Point::new(plot.x, p), Point::new(plot.x + plot.width, p), s.grid, 1.0, false);
                label(frame, text.clone(), Point::new(plot.x - 6.0, p), s.tick_size, s.muted, Horizontal::Right, Vertical::Center);
            }
        }
        if !self.value_axis.label.is_empty() {
            if horizontal {
                label(frame, self.value_axis.label.clone(), Point::new(plot.x + plot.width / 2.0, size.height - pad), s.label_size, s.text, Horizontal::Center, Vertical::Bottom);
            } else {
                label(frame, self.value_axis.label.clone(), Point::new(plot.x, plot.y - 6.0), s.label_size, s.text, Horizontal::Left, Vertical::Bottom);
            }
        }

        // Category labels and separators.
        for (i, name) in self.categories.iter().enumerate() {
            let c = band_start(i) + band / 2.0;
            if horizontal {
                let text = fit(name, s.label_size, plot.x - pad - 8.0);
                label(frame, text, Point::new(plot.x - 8.0, c), s.label_size, s.text, Horizontal::Right, Vertical::Center);
            } else {
                let text = fit(name, s.label_size, band - 4.0);
                label(frame, text, Point::new(c, plot.y + plot.height + 4.0), s.label_size, s.text, Horizontal::Center, Vertical::Top);
            }
        }

        // Reference lines (under the data).
        for r in &self.ref_lines {
            let a = at(r.value, if horizontal { plot.y } else { plot.x });
            let b = at(r.value, if horizontal { plot.y + plot.height } else { plot.x + plot.width });
            line(frame, a, b, r.color, 1.2, r.dashed);
            if let Some(text) = &r.label {
                let p = at(r.value, if horizontal { plot.y } else { plot.x + plot.width });
                if horizontal {
                    label(frame, text.clone(), Point::new(p.x + 3.0, p.y + 1.0), s.tick_size, r.color, Horizontal::Left, Vertical::Top);
                } else {
                    label(frame, text.clone(), Point::new(p.x - 3.0, p.y - 2.0), s.tick_size, r.color, Horizontal::Right, Vertical::Bottom);
                }
            }
        }

        // Layers: the dodged ones share each band.
        let dodged = self.layers.iter().filter(|l| !matches!(l, Layer::Stacked { .. })).count().max(1);
        let inner = band * 0.76;
        let slot = inner / dodged as f32;
        let mut k = 0;
        for layer in &self.layers {
            let slot_of = |i: usize, k: usize| band_start(i) + (band - inner) / 2.0 + slot * k as f32;
            match layer {
                Layer::Bars { color, values, errors, .. } => {
                    let base = ticks.min.max(0.0).min(ticks.max);
                    for (i, v) in values.iter().enumerate().take(n) {
                        let start = slot_of(i, k) + slot * 0.1;
                        let thick = slot * 0.8;
                        if let Some(v) = v {
                            let (a, b) = (at(base, start), at(*v, start + thick));
                            let rect = Rectangle::new(Point::new(a.x.min(b.x), a.y.min(b.y)), Size::new((a.x - b.x).abs().max(1.0), (a.y - b.y).abs().max(1.0)));
                            frame.fill_rectangle(rect.position(), rect.size(), Color { a: 0.85, ..*color });
                        }
                        if let Some(Some((lo, hi))) = errors.get(i) {
                            whisker(frame, at(*lo, start + thick / 2.0), at(*hi, start + thick / 2.0), (thick * 0.35).min(6.0), horizontal, s.text);
                        }
                    }
                    k += 1;
                }
                Layer::Boxes { color, samples, .. } => {
                    for (i, sample) in samples.iter().enumerate().take(n) {
                        let Some(b) = BoxStats::from_samples(sample) else { continue };
                        let start = slot_of(i, k) + slot * 0.15;
                        let thick = slot * 0.7;
                        let mid = start + thick / 2.0;
                        let cap = (thick * 0.3).min(6.0);
                        whisker(frame, at(b.whisker_lo, mid), at(b.q1, mid), cap, horizontal, *color);
                        whisker(frame, at(b.q3, mid), at(b.whisker_hi, mid), cap, horizontal, *color);
                        let (a, c) = (at(b.q1, start), at(b.q3, start + thick));
                        let rect = Rectangle::new(Point::new(a.x.min(c.x), a.y.min(c.y)), Size::new((a.x - c.x).abs().max(1.0), (a.y - c.y).abs().max(1.0)));
                        frame.fill_rectangle(rect.position(), rect.size(), Color { a: 0.35, ..*color });
                        frame.stroke(&Path::rectangle(rect.position(), rect.size()), Stroke::default().with_color(*color).with_width(1.2));
                        line(frame, at(b.median, start), at(b.median, start + thick), *color, 2.0, false);
                        for o in &b.outliers {
                            frame.stroke(&Path::circle(at(*o, mid), 2.2), Stroke::default().with_color(*color).with_width(1.0));
                        }
                    }
                    k += 1;
                }
                Layer::Points { color, values, emphasis, .. } => {
                    for (i, v) in values.iter().enumerate().take(n) {
                        let Some(v) = v else { continue };
                        let mid = slot_of(i, k) + slot / 2.0;
                        let big = emphasis.get(i).copied().unwrap_or(false);
                        if v.lo.is_finite() && v.hi.is_finite() {
                            whisker(frame, at(v.lo, mid), at(v.hi, mid), if big { 6.0 } else { 4.0 }, horizontal, *color);
                        }
                        if v.value.is_finite() {
                            let p = at(v.value, mid);
                            if big {
                                // A diamond for the summary row.
                                let r = 6.0;
                                let diamond = Path::new(|b| {
                                    b.move_to(Point::new(p.x, p.y - r));
                                    b.line_to(Point::new(p.x + r, p.y));
                                    b.line_to(Point::new(p.x, p.y + r));
                                    b.line_to(Point::new(p.x - r, p.y));
                                    b.close();
                                });
                                frame.fill(&diamond, *color);
                            } else {
                                frame.fill(&Path::circle(p, 3.5), *color);
                            }
                        }
                    }
                    k += 1;
                }
                Layer::Stacked { segments } => {
                    let start = |i: usize| band_start(i) + band * 0.15;
                    let thick = band * 0.7;
                    for i in 0..n {
                        let mut acc = 0.0;
                        for (_, color, values) in segments {
                            let v = values.get(i).copied().unwrap_or(0.0);
                            if v <= 0.0 {
                                continue;
                            }
                            let (a, b) = (at(acc, start(i)), at(acc + v, start(i) + thick));
                            let rect = Rectangle::new(Point::new(a.x.min(b.x), a.y.min(b.y)), Size::new((a.x - b.x).abs(), (a.y - b.y).abs()));
                            frame.fill_rectangle(rect.position(), rect.size(), *color);
                            acc += v;
                        }
                    }
                }
            }
        }

        // Axis line along the categories, at the value axis' start.
        let (a, b) = if horizontal {
            (Point::new(plot.x, plot.y + plot.height), Point::new(plot.x + plot.width, plot.y + plot.height))
        } else {
            (Point::new(plot.x, plot.y + plot.height), Point::new(plot.x + plot.width, plot.y + plot.height))
        };
        line(frame, a, b, s.axis, 1.0, false);
    }
}

/// An interval from `a` to `b` with end caps `cap` px long, across the value axis.
fn whisker(frame: &mut Frame, a: Point, b: Point, cap: f32, horizontal: bool, color: Color) {
    line(frame, a, b, color, 1.3, false);
    for p in [a, b] {
        if horizontal {
            line(frame, Point::new(p.x, p.y - cap), Point::new(p.x, p.y + cap), color, 1.3, false);
        } else {
            line(frame, Point::new(p.x - cap, p.y), Point::new(p.x + cap, p.y), color, 1.3, false);
        }
    }
}

impl<M> canvas::Program<M> for CategoryChart {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        self.draw_chart(&mut frame, bounds.size());
        vec![frame.into_geometry()]
    }
}

/// Height a horizontal chart needs for `categories` rows of `row` px, plus its title, legend
/// and axis (for sizing the canvas).
pub fn horizontal_height(chart: &CategoryChart, row: f32, width: f32) -> f32 {
    let s = &chart.style;
    let title = if chart.title.is_empty() { 0.0 } else { s.title_size + 6.0 };
    let legend = legend_height(&chart.legend_entries(), width - 16.0, s) + 2.0;
    let axis = s.tick_size + 6.0 + if chart.value_axis.label.is_empty() { 0.0 } else { s.label_size + 6.0 };
    16.0 + title + legend + axis + 4.0 + row * chart.categories.len().max(1) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layer_extents_include_whiskers_and_stacks() {
        let bars = Layer::Bars { name: "a".into(), color: Color::WHITE, values: vec![Some(0.5), None], errors: vec![Some((0.2, 0.9)), None] };
        assert_eq!(bars.extent(), Some((0.2, 0.9)));
        let stacked = Layer::Stacked { segments: vec![("x".into(), Color::WHITE, vec![0.3, 0.6]), ("y".into(), Color::WHITE, vec![0.7, 0.4])] };
        assert_eq!(stacked.extent(), Some((0.0, 1.0)));
        let points = Layer::Points { name: String::new(), color: Color::WHITE, values: vec![Some(Interval::new(0.1, -0.2, 0.3))], emphasis: vec![] };
        assert_eq!(points.extent(), Some((-0.2, 0.3)));
        assert!(points.legend_entries().is_empty());
    }

    #[test]
    fn horizontal_height_grows_with_rows() {
        let a = CategoryChart::new("t", vec!["a".into()]);
        let b = CategoryChart::new("t", (0..10).map(|i| i.to_string()).collect());
        assert!(horizontal_height(&b, 20.0, 400.0) - horizontal_height(&a, 20.0, 400.0) == 180.0);
    }
}
