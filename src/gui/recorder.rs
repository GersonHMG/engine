// gui/recorder.rs — Records what the field shows (robots, ball, Lua drawings) for the replay.
//
// The toolbar's REC button starts and stops it; REPLAY plays the last recording in
// replay mode. Frames are taken on every GUI tick, so the replay shows exactly what
// was on screen, at the same pace.

use std::time::Instant;

use super::field_canvas::FieldData;
use super::replay::ReplayFrame;

/// Longest recording, s; recording stops by itself after this (~18000 frames at 60 Hz).
const MAX_DURATION_S: f64 = 300.0;

#[derive(Default)]
pub struct Recorder {
    started: Option<Instant>,
    frames: Vec<ReplayFrame>,
    last: Vec<ReplayFrame>,
}

impl Recorder {
    pub fn is_recording(&self) -> bool {
        self.started.is_some()
    }

    /// Seconds since the recording started, or `None` when not recording.
    pub fn elapsed_s(&self) -> Option<f64> {
        self.started.map(|t| t.elapsed().as_secs_f64())
    }

    pub fn start(&mut self) {
        self.started = Some(Instant::now());
        self.frames.clear();
    }

    /// Stops recording; a recording with frames becomes the last recording.
    pub fn stop(&mut self) {
        if self.started.take().is_some() && !self.frames.is_empty() {
            self.last = std::mem::take(&mut self.frames);
        }
    }

    /// Adds what the field shows now, while recording.
    pub fn capture(&mut self, field: &FieldData) {
        let Some(started) = self.started else {
            return;
        };
        let elapsed = started.elapsed();
        self.frames.push(ReplayFrame {
            elapsed_ms: elapsed.as_millis() as u64,
            robots_blue: field.robots_blue.clone(),
            robots_yellow: field.robots_yellow.clone(),
            ball: field.ball,
            draws: field.lua_draw_commands.clone(),
        });
        if elapsed.as_secs_f64() >= MAX_DURATION_S {
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
        recorder.capture(&field);
        recorder.capture(&field);
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
        recorder.capture(&field);
        recorder.stop();

        recorder.start();
        recorder.stop();
        assert_eq!(recorder.last_recording().len(), 1);
    }

    #[test]
    fn capture_does_nothing_when_not_recording() {
        let field = FieldData::new(9.0, 6.0);
        let mut recorder = Recorder::default();
        recorder.capture(&field);
        recorder.stop();
        assert!(!recorder.has_recording());
    }
}
