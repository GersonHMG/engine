use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::time::Instant;

use ssl_sim::Team;

use super::field_canvas::{FieldData, LuaDrawCommand, RobotData};
use crate::sender::simulator::scenario::{BallPlacement, RobotPlacement, Scenario};

#[derive(Debug, Clone)]
pub struct ReplayFrame {
    pub elapsed_ms: u64,
    pub robots_blue: Vec<RobotData>,
    pub robots_yellow: Vec<RobotData>,
    pub ball: (f64, f64),
    /// Lua drawings on the field at this frame (empty for CSV replays).
    pub draws: Vec<LuaDrawCommand>,
}

/// Playback speeds offered in the replay toolbar.
pub const REPLAY_SPEEDS: [ReplaySpeed; 7] = [
    ReplaySpeed(0.1),
    ReplaySpeed(0.25),
    ReplaySpeed(0.5),
    ReplaySpeed(1.0),
    ReplaySpeed(2.0),
    ReplaySpeed(4.0),
    ReplaySpeed(8.0),
];

/// Replay playback speed, as a multiple of real time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReplaySpeed(pub f64);

impl fmt::Display for ReplaySpeed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}x", self.0)
    }
}

#[derive(Debug)]
pub struct ReplayState {
    pub enabled: bool,
    pub file_path: Option<String>,
    /// Set when the frames come from the toolbar's recorder instead of a CSV file.
    pub recording_label: Option<String>,
    pub is_playing: bool,
    pub frames: Vec<ReplayFrame>,
    pub frame_index: usize,
    pub elapsed_ms: u64,
    /// Playback speed, as a multiple of real time.
    pub speed: f64,
    /// Played time not yet counted in `elapsed_ms` (under 1 ms), so slow speeds do not stall.
    pending_ms: f64,
    last_tick: Instant,
}

impl Default for ReplayState {
    fn default() -> Self {
        Self {
            enabled: false,
            file_path: None,
            recording_label: None,
            is_playing: false,
            frames: Vec::new(),
            frame_index: 0,
            elapsed_ms: 0,
            speed: 1.0,
            pending_ms: 0.0,
            last_tick: Instant::now(),
        }
    }
}

impl ReplayState {
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        self.is_playing = false;
        self.last_tick = Instant::now();
    }

    pub fn has_frames(&self) -> bool {
        !self.frames.is_empty()
    }

    pub fn current_frame_elapsed_ms(&self) -> u64 {
        self.frames
            .get(self.frame_index)
            .map(|f| f.elapsed_ms)
            .unwrap_or(0)
    }

    pub fn load_and_apply(
        &mut self,
        path: String,
        field_data: &mut FieldData,
        robot_trace: &mut Vec<(f64, f64)>,
    ) -> Result<(), String> {
        self.file_path = Some(path);
        self.recording_label = None;
        self.is_playing = false;
        self.last_tick = Instant::now();

        match load_replay_frames(self.file_path.as_deref().unwrap_or_default()) {
            Ok(frames) => {
                self.frames = frames;
                self.frame_index = 0;
                self.elapsed_ms = self.frames.first().map(|f| f.elapsed_ms).unwrap_or(0);
                self.apply_current_frame(field_data, robot_trace);
                Ok(())
            }
            Err(e) => {
                self.frames.clear();
                self.frame_index = 0;
                self.elapsed_ms = 0;
                Err(e)
            }
        }
    }

    /// Loads frames recorded in the GUI (robots, ball and Lua drawings) and shows the first one.
    pub fn load_recording(
        &mut self,
        frames: Vec<ReplayFrame>,
        label: String,
        field_data: &mut FieldData,
        robot_trace: &mut Vec<(f64, f64)>,
    ) {
        self.file_path = None;
        self.recording_label = Some(label);
        self.is_playing = false;
        self.last_tick = Instant::now();
        self.frames = frames;
        self.frame_index = 0;
        self.elapsed_ms = self.frames.first().map(|f| f.elapsed_ms).unwrap_or(0);
        self.apply_current_frame(field_data, robot_trace);
    }

    pub fn start_playback(&mut self, field_data: &mut FieldData, robot_trace: &mut Vec<(f64, f64)>) {
        if !self.has_frames() {
            return;
        }

        if self.frame_index + 1 >= self.frames.len() {
            self.frame_index = 0;
            self.elapsed_ms = self.frames[0].elapsed_ms;
            self.apply_current_frame(field_data, robot_trace);
        }

        self.is_playing = true;
        self.last_tick = Instant::now();
    }

    pub fn pause(&mut self) {
        self.is_playing = false;
    }

    /// Plays when paused and pauses when playing.
    pub fn toggle_playback(&mut self, field_data: &mut FieldData, robot_trace: &mut Vec<(f64, f64)>) {
        if self.is_playing {
            self.pause();
        } else {
            self.start_playback(field_data, robot_trace);
        }
    }

    /// Where the robots and the ball are in the frame shown, as a simulator scenario.
    pub fn current_scenario(&self) -> Option<Scenario> {
        let frame = self.frames.get(self.frame_index)?;
        let placements = |robots: &[RobotData], team: Team| {
            robots
                .iter()
                .map(move |r| RobotPlacement { team, id: r.id, x: r.x, y: r.y, theta: r.theta })
                .collect::<Vec<_>>()
        };
        let mut robots = placements(&frame.robots_blue, Team::Blue);
        robots.extend(placements(&frame.robots_yellow, Team::Yellow));
        Some(Scenario { ball: BallPlacement { x: frame.ball.0, y: frame.ball.1 }, robots })
    }

    /// A scenario name for the frame shown, e.g. `attack_2v1_4520ms`.
    pub fn default_scenario_name(&self) -> String {
        let source = self
            .file_path
            .as_deref()
            .and_then(|p| std::path::Path::new(p).file_stem())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "recording".to_string());
        let source: String = source
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect();
        format!("{source}_{}ms", self.current_frame_elapsed_ms())
    }

    pub fn seek(&mut self, index: u32, field_data: &mut FieldData, robot_trace: &mut Vec<(f64, f64)>) {
        if self.frames.is_empty() {
            return;
        }

        self.is_playing = false;
        self.last_tick = Instant::now();

        let idx = (index as usize).min(self.frames.len().saturating_sub(1));
        self.frame_index = idx;
        self.elapsed_ms = self.frames[idx].elapsed_ms;
        self.apply_current_frame(field_data, robot_trace);
    }

    pub fn tick(&mut self, field_data: &mut FieldData, robot_trace: &mut Vec<(f64, f64)>) {
        field_data.vision_connected = true;

        if self.frames.is_empty() {
            field_data.robots_blue.clear();
            field_data.robots_yellow.clear();
            field_data.ball = (0.0, 0.0);
            field_data.lua_draw_commands.clear();
            return;
        }

        if self.is_playing {
            let now = Instant::now();
            self.pending_ms += now.duration_since(self.last_tick).as_secs_f64() * 1000.0 * self.speed;
            self.last_tick = now;
            let delta_ms = self.pending_ms.floor();
            self.pending_ms -= delta_ms;
            self.elapsed_ms = self.elapsed_ms.saturating_add(delta_ms as u64);

            while self.frame_index + 1 < self.frames.len()
                && self.frames[self.frame_index + 1].elapsed_ms <= self.elapsed_ms
            {
                self.frame_index += 1;
            }

            if self.frame_index + 1 >= self.frames.len() {
                self.is_playing = false;
                self.elapsed_ms = self.frames[self.frame_index].elapsed_ms;
            }
        } else {
            self.last_tick = Instant::now();
        }

        self.apply_current_frame(field_data, robot_trace);
    }

    fn apply_current_frame(&self, field_data: &mut FieldData, robot_trace: &mut Vec<(f64, f64)>) {
        if let Some(frame) = self.frames.get(self.frame_index) {
            field_data.robots_blue = frame.robots_blue.clone();
            field_data.robots_yellow = frame.robots_yellow.clone();
            field_data.ball = frame.ball;
            field_data.lua_draw_commands = frame.draws.clone();
            field_data.robot_trace.clear();
            robot_trace.clear();
        }
    }
}

#[derive(Default)]
struct ReplayFrameBuilder {
    robots_blue: HashMap<u32, RobotData>,
    robots_yellow: HashMap<u32, RobotData>,
    ball: (f64, f64),
}

pub fn load_replay_frames(path: &str) -> Result<Vec<ReplayFrame>, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read file '{}': {e}", path))?;

    let mut by_time: BTreeMap<u64, ReplayFrameBuilder> = BTreeMap::new();

    for (line_idx, line) in content.lines().enumerate() {
        if line_idx == 0 {
            // CSV header
            continue;
        }

        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let cols: Vec<&str> = line.split(',').collect();
        if cols.len() < 11 {
            continue;
        }

        let elapsed = match cols[0].trim().parse::<f64>() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let robot_id = match cols[1].trim().parse::<i32>() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let team = match cols[2].trim().parse::<i32>() {
            Ok(v) => v,
            Err(_) => continue,
        };

        let vx_cmd = match cols[3].trim().parse::<f64>() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let vy_cmd = match cols[4].trim().parse::<f64>() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let angular_cmd = match cols[5].trim().parse::<f64>() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let pos_x = match cols[6].trim().parse::<f64>() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let pos_y = match cols[7].trim().parse::<f64>() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let theta = match cols[8].trim().parse::<f64>() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let vx_actual = match cols[9].trim().parse::<f64>() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let vy_actual = match cols[10].trim().parse::<f64>() {
            Ok(v) => v,
            Err(_) => continue,
        };

        let elapsed_ms = (elapsed.max(0.0) * 1000.0).round() as u64;
        let frame = by_time.entry(elapsed_ms).or_default();

        if robot_id == -1 && team == -1 {
            frame.ball = (pos_x, pos_y);
            continue;
        }

        if robot_id < 0 {
            continue;
        }

        let robot = RobotData {
            id: robot_id as u32,
            x: pos_x,
            y: pos_y,
            theta,
            vx: vx_actual,
            vy: vy_actual,
            cmd_vx: vx_cmd,
            cmd_vy: vy_cmd,
            cmd_angular: angular_cmd,
        };

        match team {
            0 => {
                frame.robots_blue.insert(robot.id, robot);
            }
            1 => {
                frame.robots_yellow.insert(robot.id, robot);
            }
            _ => {}
        }
    }

    let mut frames = Vec::new();
    for (elapsed_ms, builder) in by_time {
        let mut robots_blue: Vec<RobotData> = builder.robots_blue.into_values().collect();
        let mut robots_yellow: Vec<RobotData> = builder.robots_yellow.into_values().collect();
        robots_blue.sort_by_key(|r| r.id);
        robots_yellow.sort_by_key(|r| r.id);
        frames.push(ReplayFrame {
            elapsed_ms,
            robots_blue,
            robots_yellow,
            ball: builder.ball,
            draws: Vec::new(),
        });
    }

    Ok(frames)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn robot(id: u32, x: f64, y: f64, theta: f64) -> RobotData {
        RobotData { id, x, y, theta, vx: 0.0, vy: 0.0, cmd_vx: 0.0, cmd_vy: 0.0, cmd_angular: 0.0 }
    }

    fn frame(elapsed_ms: u64, ball: (f64, f64)) -> ReplayFrame {
        ReplayFrame {
            elapsed_ms,
            robots_blue: vec![robot(0, -1.0, 0.5, 0.3)],
            robots_yellow: vec![robot(2, 1.5, -0.5, 3.0)],
            ball,
            draws: Vec::new(),
        }
    }

    fn replay_with_frames() -> (ReplayState, FieldData, Vec<(f64, f64)>) {
        let mut field = FieldData::new(9.0, 6.0);
        let mut trace = Vec::new();
        let mut replay = ReplayState::default();
        let frames = vec![frame(0, (0.0, 0.0)), frame(4520, (0.25, 0.75))];
        replay.load_recording(frames, "test".to_string(), &mut field, &mut trace);
        (replay, field, trace)
    }

    #[test]
    fn toggle_playback_plays_then_pauses() {
        let (mut replay, mut field, mut trace) = replay_with_frames();
        replay.toggle_playback(&mut field, &mut trace);
        assert!(replay.is_playing);
        replay.toggle_playback(&mut field, &mut trace);
        assert!(!replay.is_playing);
    }

    #[test]
    fn scenario_is_the_frame_shown() {
        let (mut replay, mut field, mut trace) = replay_with_frames();
        replay.seek(1, &mut field, &mut trace);
        let scenario = replay.current_scenario().unwrap();
        assert_eq!((scenario.ball.x, scenario.ball.y), (0.25, 0.75));
        assert_eq!(
            scenario.robots,
            vec![
                RobotPlacement { team: Team::Blue, id: 0, x: -1.0, y: 0.5, theta: 0.3 },
                RobotPlacement { team: Team::Yellow, id: 2, x: 1.5, y: -0.5, theta: 3.0 },
            ]
        );
        assert_eq!(replay.default_scenario_name(), "recording_4520ms");
    }

    #[test]
    fn no_scenario_without_frames() {
        assert!(ReplayState::default().current_scenario().is_none());
    }
}
