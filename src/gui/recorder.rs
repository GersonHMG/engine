// gui/recorder.rs — Records what the field shows (robots, ball, Lua drawings) for the replay.
//
// The toolbar's REC button starts and stops it; REPLAY plays the last recording in
// replay mode. Frames are taken on every GUI tick, so the replay shows exactly what
// was on screen. Frames are timed by the simulator's clock while it runs, so the
// replay at 1x shows the play at real speed whatever speed the simulator ran at;
// without the simulator they are timed by the wall clock.

use std::time::Instant;

use super::field_canvas::FieldData;
use super::replay::ReplayFrame;

/// Longest recording, s; recording stops by itself after this (~18000 frames at 60 Hz).
const MAX_DURATION_S: f64 = 300.0;

#[derive(Default)]
pub struct Recorder {
    /// Clock of the recording in progress; `None` when not recording.
    clock: Option<Clock>,
    frames: Vec<ReplayFrame>,
    last: Vec<ReplayFrame>,
}

/// Recorded time, advanced by simulated time when there is one, else by wall time.
struct Clock {
    elapsed_s: f64,
    last_wall: Instant,
    last_sim: Option<f64>,
}

impl Clock {
    fn new() -> Self {
        Self { elapsed_s: 0.0, last_wall: Instant::now(), last_sim: None }
    }

    fn advance(&mut self, sim_time: Option<f64>) {
        let now = Instant::now();
        let wall_dt = now.duration_since(self.last_wall).as_secs_f64();
        let dt = match (self.last_sim, sim_time) {
            // A simulator reset moves its time back: count nothing for that tick.
            (Some(last), Some(time)) => (time - last).max(0.0),
            // The simulator was just switched on or off.
            (None, Some(_)) | (Some(_), None) => 0.0,
            (None, None) => wall_dt,
        };
        self.elapsed_s += dt;
        self.last_wall = now;
        self.last_sim = sim_time;
    }
}

impl Recorder {
    pub fn is_recording(&self) -> bool {
        self.clock.is_some()
    }

    /// Recorded seconds so far, or `None` when not recording.
    pub fn elapsed_s(&self) -> Option<f64> {
        self.clock.as_ref().map(|c| c.elapsed_s)
    }

    pub fn start(&mut self) {
        self.clock = Some(Clock::new());
        self.frames.clear();
    }

    /// Stops recording; a recording with frames becomes the last recording.
    pub fn stop(&mut self) {
        if self.clock.take().is_some() && !self.frames.is_empty() {
            self.last = std::mem::take(&mut self.frames);
        }
    }

    /// Adds what the field shows now, while recording. `sim_time` is the
    /// simulator's time, s, when the simulator is running.
    pub fn capture(&mut self, field: &FieldData, sim_time: Option<f64>) {
        let Some(clock) = self.clock.as_mut() else {
            return;
        };
        let first = self.frames.is_empty();
        clock.advance(sim_time);
        if first {
            // Time starts at the first frame.
            clock.elapsed_s = 0.0;
        }
        let elapsed_s = clock.elapsed_s;
        self.frames.push(ReplayFrame {
            elapsed_ms: (elapsed_s * 1000.0).round() as u64,
            robots_blue: field.robots_blue.clone(),
            robots_yellow: field.robots_yellow.clone(),
            ball: field.ball,
            draws: field.lua_draw_commands.clone(),
        });
        if elapsed_s >= MAX_DURATION_S {
            self.stop();
        }
    }

    pub fn has_recording(&self) -> bool {
        !self.last.is_empty()
    }

    /// The last finished recording.
    pub fn last_recording(&self) -> &[ReplayFrame] {
        &self.last
    }

    /// Length of the last finished recording, s.
    pub fn last_duration_s(&self) -> f64 {
        self.last.last().map_or(0.0, |f| f.elapsed_ms as f64 / 1000.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stop_keeps_frames_as_last_recording() {
        let field = FieldData::new(9.0, 6.0);
        let mut recorder = Recorder::default();
        assert!(!recorder.has_recording());

        recorder.start();
        recorder.capture(&field, None);
        recorder.capture(&field, None);
        assert!(recorder.is_recording());
        assert!(!recorder.has_recording());

        recorder.stop();
        assert!(!recorder.is_recording());
        assert_eq!(recorder.last_recording().len(), 2);
    }

    #[test]
    fn empty_recording_keeps_the_previous_one() {
        let field = FieldData::new(9.0, 6.0);
        let mut recorder = Recorder::default();
        recorder.start();
        recorder.capture(&field, None);
        recorder.stop();

        recorder.start();
        recorder.stop();
        assert_eq!(recorder.last_recording().len(), 1);
    }

    #[test]
    fn frames_are_timed_by_the_simulator_clock() {
        let field = FieldData::new(9.0, 6.0);
        let mut recorder = Recorder::default();
        recorder.start();
        recorder.capture(&field, Some(10.0));
        recorder.capture(&field, Some(10.5));
        recorder.capture(&field, Some(2.0)); // simulator reset
        recorder.capture(&field, Some(2.25));
        recorder.stop();
        let times: Vec<u64> = recorder.last_recording().iter().map(|f| f.elapsed_ms).collect();
        assert_eq!(times, [0, 500, 500, 750]);
    }

    #[test]
    fn capture_does_nothing_when_not_recording() {
        let field = FieldData::new(9.0, 6.0);
        let mut recorder = Recorder::default();
        recorder.capture(&field, None);
        recorder.stop();
        assert!(!recorder.has_recording());
    }
}
