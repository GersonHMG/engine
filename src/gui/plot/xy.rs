// gui/plot/xy.rs — Numeric x/y charts: lines, steps, scatter, interval bands
// (see gui/plot/mod.rs)

use iced::alignment::{Horizontal, Vertical};
use iced::widget::canvas::{self, Frame, Geometry, Path, Stroke};
use iced::widget::Canvas;
use iced::{mouse, Color, Element, Length, Point, Rectangle, Renderer, Size, Theme};

use super::scale::{format_tick, Linear};
use super::{label, legend, line, text_width, Axis, RefLine, Style};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeriesKind {
    Line,
    /// Holds each y until the next x (empirical distributions, cumulative counts).
    Step,
    Scatter,
}

#[derive(Debug, Clone)]
pub struct Series {
    pub name: String,
    pub color: Color,
    pub kind: SeriesKind,
    pub points: Vec<(f64, f64)>,
    /// Shaded (x, lo, hi) band under the series.
    pub band: Vec<(f64, f64, f64)>,
}

impl Series {
    pub fn new(name: impl Into<String>, color: Color, kind: SeriesKind, points: Vec<(f64, f64)>) -> Self {
        Self { name: name.into(), color, kind, points, band: Vec::new() }
    }

    pub fn band(mut self, band: Vec<(f64, f64, f64)>) -> Self {
        self.band = band;
        self
    }
}

#[derive(Debug, Clone, Default)]
pub struct XyChart {
    pub title: String,
    pub x_axis: Axis,
    pub y_axis: Axis,
    pub series: Vec<Series>,
    /// Lines at y = value.
    pub y_lines: Vec<RefLine>,
    /// Lines at x = value.
    pub x_lines: Vec<RefLine>,
    pub legend: bool,
    pub empty_text: String,
    pub style: Style,
}

impl XyChart {
    pub fn new(title: impl Into<String>, x_axis: Axis, y_axis: Axis) -> Self {
        Self { title: title.into(), x_axis, y_axis, legend: true, empty_text: "No data".into(), ..Self::default() }
    }

    pub fn series(mut self, series: Series) -> Self {
        self.series.push(series);
        self
    }

    pub fn y_line(mut self, line: RefLine) -> Self {
        self.y_lines.push(line);
        self
    }

    pub fn x_line(mut self, line: RefLine) -> Self {
        self.x_lines.push(line);
        self
    }

    pub fn view<'a, M: 'a>(&'a self, height: f32) -> Element<'a, M> {
        Canvas::new(self).width(Length::Fill).height(Length::Fixed(height)).into()
    }

    fn extent(&self, x: bool) -> Option<(f64, f64)> {
        let mut values: Vec<f64> = Vec::new();
        for s in &self.series {
            if x {
                values.extend(s.points.iter().map(|p| p.0));
                values.extend(s.band.iter().map(|b| b.0));
            } else {
                values.extend(s.points.iter().map(|p| p.1));
                values.extend(s.band.iter().flat_map(|b| [b.1, b.2]));
            }
        }
        let lines = if x { &self.x_lines } else { &self.y_lines };
        values.extend(lines.iter().map(|l| l.value));
        let values: Vec<f64> = values.into_iter().filter(|v| v.is_finite()).collect();
        Some((values.iter().copied().reduce(f64::min)?, values.iter().copied().reduce(f64::max)?))
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
        let entries: Vec<(String, Color)> = if self.legend {
            self.series.iter().filter(|s| !s.name.is_empty()).map(|s| (s.name.clone(), s.color)).collect()
        } else {
            Vec::new()
        };
        if !entries.is_empty() {
            top += legend(frame, &entries, Point::new(pad, top), size.width - 2.0 * pad, s) + 2.0;
        }
        if self.series.iter().all(|s| s.points.is_empty()) {
            label(frame, self.empty_text.clone(), Point::new(size.width / 2.0, (top + size.height) / 2.0), s.label_size, s.muted, Horizontal::Center, Vertical::Center);
            return;
        }

        let y_label_h = if self.y_axis.label.is_empty() { 0.0 } else { s.label_size + 6.0 };
        let x_label_h = if self.x_axis.label.is_empty() { 0.0 } else { s.label_size + 6.0 };
        let y_ticks = self.y_axis.ticks(self.extent(false), ((size.height / 50.0) as usize).clamp(2, 8));
        let y_text: Vec<String> = y_ticks.values.iter().map(|v| format_tick(*v, y_ticks.step, self.y_axis.format)).collect();
        let left = pad + y_text.iter().map(|t| text_width(t, s.tick_size)).fold(0.0, f32::max) + 6.0;
        let x_ticks = self.x_axis.ticks(self.extent(true), (((size.width - left) / 70.0) as usize).clamp(2, 10));
        let x_text: Vec<String> = x_ticks.values.iter().map(|v| format_tick(*v, x_ticks.step, self.x_axis.format)).collect();
        let right = pad + x_text.last().map_or(0.0, |t| text_width(t, s.tick_size) / 2.0);
        let plot_top = top + y_label_h + 4.0;
        let plot_bottom = size.height - pad - x_label_h - s.tick_size - 6.0;
        let plot = Rectangle::new(Point::new(left, plot_top), Size::new((size.width - right - left).max(10.0), (plot_bottom - plot_top).max(10.0)));
        let sx = Linear::new((x_ticks.min, x_ticks.max), (plot.x, plot.x + plot.width));
        let sy = Linear::new((y_ticks.min, y_ticks.max), (plot.y + plot.height, plot.y));
        let at = |x: f64, y: f64| Point::new(sx.map(x), sy.map(y));

        for (v, t) in y_ticks.values.iter().zip(&y_text) {
            let p = sy.map(*v);
            line(frame, Point::new(plot.x, p), Point::new(plot.x + plot.width, p), s.grid, 1.0, false);
            label(frame, t.clone(), Point::new(plot.x - 6.0, p), s.tick_size, s.muted, Horizontal::Right, Vertical::Center);
        }
        for (v, t) in x_ticks.values.iter().zip(&x_text) {
            let p = sx.map(*v);
            line(frame, Point::new(p, plot.y), Point::new(p, plot.y + plot.height), s.grid, 1.0, false);
            label(frame, t.clone(), Point::new(p, plot.y + plot.height + 4.0), s.tick_size, s.muted, Horizontal::Center, Vertical::Top);
        }
        if !self.y_axis.label.is_empty() {
            label(frame, self.y_axis.label.clone(), Point::new(plot.x, plot.y - 6.0), s.label_size, s.text, Horizontal::Left, Vertical::Bottom);
        }
        if !self.x_axis.label.is_empty() {
            label(frame, self.x_axis.label.clone(), Point::new(plot.x + plot.width / 2.0, size.height - pad), s.label_size, s.text, Horizontal::Center, Vertical::Bottom);
        }
        for r in &self.y_lines {
            line(frame, at(x_ticks.min, r.value), at(x_ticks.max, r.value), r.color, 1.2, r.dashed);
            if let Some(text) = &r.label {
                label(frame, text.clone(), Point::new(plot.x + plot.width - 3.0, sy.map(r.value) - 2.0), s.tick_size, r.color, Horizontal::Right, Vertical::Bottom);
            }
        }
        for r in &self.x_lines {
            line(frame, at(r.value, y_ticks.min), at(r.value, y_ticks.max), r.color, 1.2, r.dashed);
            if let Some(text) = &r.label {
                label(frame, text.clone(), Point::new(sx.map(r.value) + 3.0, plot.y + 1.0), s.tick_size, r.color, Horizontal::Left, Vertical::Top);
            }
        }

        for series in &self.series {
            if series.band.len() >= 2 {
                let band = Path::new(|b| {
                    let (x, lo, _) = series.band[0];
                    b.move_to(at(x, lo));
                    for &(x, lo, _) in &series.band[1..] {
                        b.line_to(at(x, lo));
                    }
                    for &(x, _, hi) in series.band.iter().rev() {
                        b.line_to(at(x, hi));
                    }
                    b.close();
                });
                frame.fill(&band, Color { a: 0.18, ..series.color });
            }
            let stroke = Stroke::default().with_color(series.color).with_width(1.8);
            match series.kind {
                SeriesKind::Line | SeriesKind::Step if series.points.len() >= 2 => {
                    let path = Path::new(|b| {
                        let (x0, y0) = series.points[0];
                        b.move_to(at(x0, y0));
                        let mut prev_y = y0;
                        for &(x, y) in &series.points[1..] {
                            if series.kind == SeriesKind::Step {
                                b.line_to(at(x, prev_y));
                            }
                            b.line_to(at(x, y));
                            prev_y = y;
                        }
                    });
                    frame.stroke(&path, stroke);
                }
                _ => {
                    for &(x, y) in &series.points {
                        frame.fill(&Path::circle(at(x, y), 2.6), series.color);
                    }
                }
            }
        }
        line(frame, Point::new(plot.x, plot.y + plot.height), Point::new(plot.x + plot.width, plot.y + plot.height), s.axis, 1.0, false);
        line(frame, Point::new(plot.x, plot.y), Point::new(plot.x, plot.y + plot.height), s.axis, 1.0, false);
    }
}

impl<M> canvas::Program<M> for XyChart {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        self.draw_chart(&mut frame, bounds.size());
        vec![frame.into_geometry()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extents_include_bands_and_reference_lines() {
        let chart = XyChart::new("t", Axis::new("x"), Axis::new("y"))
            .series(Series::new("a", Color::WHITE, SeriesKind::Line, vec![(0.0, 1.0), (2.0, 3.0)]).band(vec![(0.0, 0.5, 1.5), (2.0, 2.0, 4.0)]))
            .y_line(RefLine::new(-1.0))
            .x_line(RefLine::new(5.0));
        assert_eq!(chart.extent(true), Some((0.0, 5.0)));
        assert_eq!(chart.extent(false), Some((-1.0, 4.0)));
        assert_eq!(XyChart::default().extent(true), None);
    }
}
