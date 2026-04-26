// logger.rs — CSV frame logger
// Port of logger/logger.cpp

use crate::types::{BallState, RobotCommand, RobotState};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::time::Instant;
use tracing::{debug, warn};

pub struct Logger {
    writer: Option<BufWriter<File>>,
    is_logging: bool,
    timer: Instant,
}

impl Logger {
    pub fn new() -> Self {
        Self {
            writer: None,
            is_logging: false,
            timer: Instant::now(),
        }
    }

    pub fn start_logging(&mut self, filename: Option<&str>) {
        if self.is_logging {
            return;
        }

        let file_path = match filename {
            Some(f) if !f.is_empty() => f.to_string(),
            _ => {
                let _ = fs::create_dir_all("logs");
                let timestamp = chrono_like_timestamp();
                format!("logs/log_{timestamp}.csv")
            }
        };

        match File::create(&file_path) {
            Ok(file) => {
                let mut writer = BufWriter::new(file);
                let _ = writeln!(
                    writer,
                    "ElapsedTime,RobotID,Team,Vx_Command,Vy_Command,Angular_Command,Pos_X,Pos_Y,Orientation,Vx_Actual,Vy_Actual"
                );
                self.writer = Some(writer);
                self.timer = Instant::now();
                self.is_logging = true;
                debug!("Logger started: {file_path}");
            }
            Err(e) => {
                warn!("Failed to open log file {file_path}: {e}");
            }
        }
    }

    pub fn stop_logging(&mut self) {
        if let Some(ref mut w) = self.writer {
            let _ = w.flush();
        }
        self.writer = None;
        self.is_logging = false;
        debug!("Logger stopped.");
    }

    pub fn log_frame(
        &mut self,
        blue_robots: &[RobotState],
        yellow_robots: &[RobotState],
        ball: &BallState,
        command_map: &HashMap<i32, RobotCommand>,
    ) {
        if !self.is_logging {
            return;
        }

        let writer = match self.writer.as_mut() {
            Some(w) => w,
            None => return,
        };

        let elapsed = self.timer.elapsed().as_secs_f64();
        let num_robots = 6;

        // Build a quick lookup from the receiver snapshot.
        let mut robot_lookup: HashMap<(i32, i32), &RobotState> = HashMap::new();
        for robot in blue_robots {
            robot_lookup.insert((robot.id, 0), robot);
        }
        for robot in yellow_robots {
            robot_lookup.insert((robot.id, 1), robot);
        }

        for team in 0..=1 {
            for id in 0..num_robots {
                let state = robot_lookup.get(&(id, team)).copied();
                let (pos_x, pos_y, orientation, vx_actual, vy_actual) = if let Some(s) = state {
                    (s.position.x, s.position.y, s.orientation, s.velocity.x, s.velocity.y)
                } else {
                    (0.0, 0.0, 0.0, 0.0, 0.0)
                };

                let (vx_cmd, vy_cmd, angular_cmd) = command_map
                    .get(&id)
                    .filter(|c| c.id == id && c.team == team)
                    .map(|c| (c.motion.vx.unwrap_or(0.0), c.motion.vy.unwrap_or(0.0), c.motion.angular.unwrap_or(0.0)))
                    .unwrap_or((0.0, 0.0, 0.0));

                let _ = writeln!(
                    writer,
                    "{elapsed:.3},{id},{team},{vx_cmd},{vy_cmd},{angular_cmd},{pos_x},{pos_y},{orientation},{vx_actual},{vy_actual}"
                );
            }
        }

        let _ = writeln!(
            writer,
            "{elapsed:.3},-1,-1,0,0,0,{},{},0,{},{}",
            ball.position.x, ball.position.y, ball.velocity.x, ball.velocity.y
        );

        let _ = writer.flush();
    }

    pub fn is_logging(&self) -> bool {
        self.is_logging
    }
}

impl Drop for Logger {
    fn drop(&mut self) {
        self.stop_logging();
    }
}

/// Generate a timestamp string like "20260218_115801"
fn chrono_like_timestamp() -> String {
    use std::time::SystemTime;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    // Simple epoch-based filename (not locale-aware, but functional)
    format!("{now}")
}
