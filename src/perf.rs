// perf.rs — How fast the engine loop runs
//
// Each tick records how long Lua `process()` took and how long the whole tick
// took (everything before the sleep). Once a second the window is summed up
// into `PerfStats`, shown in the GUI's bottom bar and in MCP's get_state.
// `RunStats` keeps every tick of a run, for the headless summary.

use serde::Serialize;
use std::time::{Duration, Instant};

/// Real-time budget of one tick: the engine runs at 60 Hz.
pub const BUDGET_MS: f64 = 1000.0 / 60.0;

/// One second of the engine loop.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct PerfStats {
    /// Mean and worst time of Lua `process()`, ms (None: no tick ran it).
    pub lua_avg_ms: Option<f64>,
    pub lua_max_ms: Option<f64>,
    /// Mean and worst time of a whole tick, before the sleep, ms.
    pub tick_avg_ms: f64,
    pub tick_max_ms: f64,
    /// Mean tick time as a share of the 16.7 ms budget, %.
    pub budget_pct: f64,
    /// Ticks per second that took longer than the budget.
    pub overruns: u32,
    /// Ticks completed per real second.
    pub loop_hz: f64,
    /// Memory the Lua state uses, MB.
    pub lua_mem_mb: f64,
}

/// Collects ticks and closes a window every second.
pub struct LoopStats {
    window_start: Instant,
    ticks: u32,
    tick_sum: f64,
    tick_max: f64,
    overruns: u32,
    lua_ticks: u32,
    lua_sum: f64,
    lua_max: f64,
}

impl Default for LoopStats {
    fn default() -> Self {
        Self::new(Instant::now())
    }
}

impl LoopStats {
    fn new(now: Instant) -> Self {
        Self {
            window_start: now,
            ticks: 0,
            tick_sum: 0.0,
            tick_max: 0.0,
            overruns: 0,
            lua_ticks: 0,
            lua_sum: 0.0,
            lua_max: 0.0,
        }
    }

    /// Records one tick. `lua`: time of `process()`, if the tick ran it.
    pub fn record(&mut self, tick: Duration, lua: Option<Duration>) {
        let tick_ms = ms(tick);
        self.ticks += 1;
        self.tick_sum += tick_ms;
        self.tick_max = self.tick_max.max(tick_ms);
        if tick_ms > BUDGET_MS {
            self.overruns += 1;
        }
        if let Some(lua) = lua {
            let lua_ms = ms(lua);
            self.lua_ticks += 1;
            self.lua_sum += lua_ms;
            self.lua_max = self.lua_max.max(lua_ms);
        }
    }

    /// The last second's stats once a second has passed, starting a new window.
    pub fn take_if_due(&mut self, now: Instant, lua_mem_bytes: usize) -> Option<PerfStats> {
        let elapsed = now.duration_since(self.window_start).as_secs_f64();
        if elapsed < 1.0 || self.ticks == 0 {
            return None;
        }
        let tick_avg = self.tick_sum / f64::from(self.ticks);
        let has_lua = self.lua_ticks > 0;
        let stats = PerfStats {
            lua_avg_ms: has_lua.then(|| self.lua_sum / f64::from(self.lua_ticks)),
            lua_max_ms: has_lua.then_some(self.lua_max),
            tick_avg_ms: tick_avg,
            tick_max_ms: self.tick_max,
            budget_pct: tick_avg / BUDGET_MS * 100.0,
            overruns: self.overruns,
            loop_hz: f64::from(self.ticks) / elapsed,
            lua_mem_mb: lua_mem_bytes as f64 / (1024.0 * 1024.0),
        };
        *self = Self::new(now);
        Some(stats)
    }
}

/// Every tick of a run, for a summary at the end.
#[derive(Default)]
pub struct RunStats {
    lua_ms: Vec<f64>,
    tick_ms: Vec<f64>,
}

impl RunStats {
    pub fn record(&mut self, tick: Duration, lua: Duration) {
        self.tick_ms.push(ms(tick));
        self.lua_ms.push(ms(lua));
    }

    /// One line: Lua time mean/p95/max, tick time mean/max, overruns.
    pub fn summary(&self) -> String {
        if self.tick_ms.is_empty() {
            return "perf: no ticks".to_string();
        }
        let lua = Spread::of(&self.lua_ms);
        let tick = Spread::of(&self.tick_ms);
        let overruns = self.tick_ms.iter().filter(|&&t| t > BUDGET_MS).count();
        format!(
            "perf: lua process() avg {:.2} ms, p95 {:.2} ms, max {:.2} ms | tick avg {:.2} ms ({:.0}% of {:.1} ms), max {:.2} ms | {overruns} of {} ticks over budget",
            lua.avg, lua.p95, lua.max, tick.avg, tick.avg / BUDGET_MS * 100.0, BUDGET_MS, tick.max, self.tick_ms.len(),
        )
    }
}

struct Spread {
    avg: f64,
    p95: f64,
    max: f64,
}

impl Spread {
    fn of(values: &[f64]) -> Self {
        let mut sorted = values.to_vec();
        sorted.sort_by(f64::total_cmp);
        let n = sorted.len();
        Self {
            avg: sorted.iter().sum::<f64>() / n as f64,
            p95: sorted[((n as f64 * 0.95).ceil() as usize).clamp(1, n) - 1],
            max: sorted[n - 1],
        }
    }
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sums_up_one_second() {
        let start = Instant::now();
        let mut stats = LoopStats::new(start);
        stats.record(Duration::from_millis(4), Some(Duration::from_millis(2)));
        stats.record(Duration::from_millis(20), Some(Duration::from_millis(10)));
        stats.record(Duration::from_millis(6), None);

        assert_eq!(stats.take_if_due(start + Duration::from_millis(500), 0), None);
        let s = stats.take_if_due(start + Duration::from_secs(1), 2 * 1024 * 1024).unwrap();
        assert_eq!(s.lua_avg_ms, Some(6.0));
        assert_eq!(s.lua_max_ms, Some(10.0));
        assert_eq!(s.tick_avg_ms, 10.0);
        assert_eq!(s.tick_max_ms, 20.0);
        assert_eq!(s.overruns, 1);
        assert_eq!(s.loop_hz, 3.0);
        assert_eq!(s.lua_mem_mb, 2.0);
        assert!((s.budget_pct - 60.0).abs() < 1e-9);

        // The next window starts empty.
        assert_eq!(stats.take_if_due(start + Duration::from_secs(3), 0), None);
    }

    #[test]
    fn no_lua_time_without_a_script() {
        let start = Instant::now();
        let mut stats = LoopStats::new(start);
        stats.record(Duration::from_millis(1), None);
        let s = stats.take_if_due(start + Duration::from_secs(1), 0).unwrap();
        assert_eq!(s.lua_avg_ms, None);
    }

    #[test]
    fn run_summary_has_percentiles() {
        let mut run = RunStats::default();
        for i in 1..=100 {
            run.record(Duration::from_millis(i), Duration::from_millis(i));
        }
        let line = run.summary();
        assert!(line.contains("p95 95.00 ms"), "{line}");
        assert!(line.contains("max 100.00 ms"), "{line}");
        assert!(line.contains("84 of 100 ticks over budget"), "{line}");
    }
}
