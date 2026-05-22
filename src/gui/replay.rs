use std::collections::{BTreeMap, HashMap};
use std::time::Instant;

use super::field_canvas::{FieldData, RobotData};

#[derive(Debug, Clone)]
pub struct ReplayFrame {
    pub elapsed_ms: u64,
    pub robots_blue: Vec<RobotData>,
    pub robots_yellow: Vec<RobotData>,
    pub ball: (f64, f64),
}

#[derive(Debug)]
pub struct ReplayState {
    pub enabled: bool,
    pub file_path: Option<String>,
    pub is_playing: bool,
    pub frames: Vec<ReplayFrame>,
    pub frame_index: usize,
    pub elapsed_ms: u64,
    last_tick: Instant,
}

impl Default for ReplayState {
    fn default() -> Self {
        Self {
            enabled: false,
            file_path: None,
            is_playing: false,
            frames: Vec::new(),
            frame_index: 0,
            elapsed_ms: 0,
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
        field_data.lua_draw_commands.clear();

        if self.frames.is_empty() {
            field_data.robots_blue.clear();
            field_data.robots_yellow.clear();
            field_data.ball = (0.0, 0.0);
            return;
        }

        if self.is_playing {
            let now = Instant::now();
            let delta_ms = now.duration_since(self.last_tick).as_millis() as u64;
            self.last_tick = now;
            self.elapsed_ms = self.elapsed_ms.saturating_add(delta_ms);

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
        });
    }

    Ok(frames)
}
