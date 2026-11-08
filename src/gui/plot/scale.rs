// gui/plot/scale.rs — Linear scales and "nice" axis ticks
//
// Ticks follow Heckbert's nice numbers ("Nice numbers for graph labels", Graphics Gems,
// 1990): steps of 1, 2 or 5 × 10^k, and the axis widened to whole steps.

/// How tick labels read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TickFormat {
    #[default]
    Number,
    /// 0.25 -> "25%".
    Percent,
    /// Seconds: "3 s".
    Seconds,
}

/// Maps a value domain onto a pixel range (either may be reversed).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Linear {
    pub d0: f64,
    pub d1: f64,
    pub r0: f32,
    pub r1: f32,
}

impl Linear {
    pub fn new(domain: (f64, f64), range: (f32, f32)) -> Self {
        Self { d0: domain.0, d1: domain.1, r0: range.0, r1: range.1 }
    }

    pub fn map(&self, v: f64) -> f32 {
        let span = self.d1 - self.d0;
        let t = if span.abs() < f64::EPSILON { 0.5 } else { (v - self.d0) / span };
        self.r0 + (self.r1 - self.r0) * t as f32
    }
}

/// The nice number near `x` (> 0): rounded, or the next one up (`round` = false).
pub fn nice_number(x: f64, round: bool) -> f64 {
    if x <= 0.0 || !x.is_finite() {
        return 1.0;
    }
    let exp = x.log10().floor();
    let f = x / 10f64.powf(exp);
    let nice = if round {
        if f < 1.5 { 1.0 } else if f < 3.0 { 2.0 } else if f < 7.0 { 5.0 } else { 10.0 }
    } else if f <= 1.0 {
        1.0
    } else if f <= 2.0 {
        2.0
    } else if f <= 5.0 {
        5.0
    } else {
        10.0
    };
    nice * 10f64.powf(exp)
}

/// Axis ticks covering [min, max] with about `target` intervals.
#[derive(Debug, Clone, PartialEq)]
pub struct Ticks {
    pub min: f64,
    pub max: f64,
    pub step: f64,
    pub values: Vec<f64>,
}

pub fn nice_ticks(min: f64, max: f64, target: usize) -> Ticks {
    let (mut min, mut max) = if min <= max { (min, max) } else { (max, min) };
    if !min.is_finite() || !max.is_finite() {
        min = 0.0;
        max = 1.0;
    }
    if (max - min).abs() < 1e-12 {
        let pad = if min.abs() > 1e-12 { min.abs() * 0.1 } else { 1.0 };
        min -= pad;
        max += pad;
    }
    let range = nice_number(max - min, false);
    let step = nice_number(range / target.max(1) as f64, true);
    let lo = (min / step).floor() * step;
    let hi = (max / step).ceil() * step;
    let count = ((hi - lo) / step).round() as usize;
    // Clean up float noise (0.30000000000000004 -> 0.3, -0 -> 0).
    let scale = 10f64.powi(decimals(step) as i32 + 2);
    let clean = |v: f64| {
        let v = (v * scale).round() / scale;
        if v.abs() < step * 1e-9 { 0.0 } else { v }
    };
    let values = (0..=count).map(|i| clean(lo + i as f64 * step)).collect();
    Ticks { min: clean(lo), max: clean(hi), step: clean(step), values }
}

/// Decimals that show ticks `step` apart without noise.
fn decimals(step: f64) -> usize {
    if step <= 0.0 || !step.is_finite() {
        return 0;
    }
    (-step.log10().floor()).max(0.0) as usize
}

/// A tick label.
pub fn format_tick(v: f64, step: f64, format: TickFormat) -> String {
    match format {
        TickFormat::Number => format!("{:.*}", decimals(step), v),
        TickFormat::Percent => format!("{:.*}%", decimals(step * 100.0), v * 100.0),
        TickFormat::Seconds => format!("{:.*} s", decimals(step), v),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nice_numbers() {
        assert_eq!(nice_number(0.83, true), 1.0);
        assert_eq!(nice_number(2.4, true), 2.0);
        assert_eq!(nice_number(4.0, true), 5.0);
        assert_eq!(nice_number(0.034, true), 0.05);
        assert_eq!(nice_number(1.2, false), 2.0);
        assert_eq!(nice_number(6.0, false), 10.0);
    }

    #[test]
    fn ticks_cover_the_range_in_nice_steps() {
        let t = nice_ticks(0.0, 1.0, 5);
        assert_eq!(t.step, 0.2);
        assert_eq!(t.values, vec![0.0, 0.2, 0.4, 0.6, 0.8, 1.0]);

        let t = nice_ticks(-0.37, 0.52, 5);
        assert_eq!((t.min, t.max, t.step), (-0.4, 0.6, 0.2));
        assert!(t.values.contains(&0.0));

        let t = nice_ticks(3.0, 3.0, 4);
        assert!(t.min < 3.0 && t.max > 3.0);
        let t = nice_ticks(5.0, 1.0, 4);
        assert!(t.min <= 1.0 && t.max >= 5.0);
    }

    #[test]
    fn labels_carry_just_enough_decimals() {
        assert_eq!(format_tick(0.4, 0.2, TickFormat::Number), "0.4");
        assert_eq!(format_tick(10.0, 5.0, TickFormat::Number), "10");
        assert_eq!(format_tick(0.25, 0.25, TickFormat::Percent), "25%");
        assert_eq!(format_tick(0.025, 0.005, TickFormat::Percent), "2.5%");
        assert_eq!(format_tick(-0.2, 0.1, TickFormat::Percent), "-20%");
        assert_eq!(format_tick(3.0, 1.0, TickFormat::Seconds), "3 s");
    }

    #[test]
    fn linear_maps_and_reverses() {
        let s = Linear::new((0.0, 10.0), (100.0, 0.0));
        assert_eq!(s.map(0.0), 100.0);
        assert_eq!(s.map(10.0), 0.0);
        assert_eq!(s.map(2.5), 75.0);
    }
}
