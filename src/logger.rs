// logger.rs — CSV frame logger
// Port of logger/logger.cpp

use crate::radio::Radio;
use crate::world::World;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;
use tracing::{debug, warn};

pub struct Logger {
    world: Arc<RwLock<World>>,
    radio: Arc<Mutex<Radio>>,
    writer: Option<BufWriter<File>>,
    is_logging: bool,
    timer: Instant,
}

impl Logger {
    pub fn new(world: Arc<RwLock<World>>, radio: Arc<Mutex<Radio>>) -> Self {
        Self {
            world,
            radio,
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

    pub fn log_frame(&mut self) {
        if !self.is_logging {
            return;
        }

        let writer = match self.writer.as_mut() {
            Some(w) => w,
            None => return,
        };

        let elapsed = self.timer.elapsed().as_secs_f64();
        let num_robots = 6;

        // Get command map snapshot
        let command_map = if let Ok(r) = self.radio.lock() {
            r.get_command_map().clone()
        } else {
            return;
        };

        let world = if let Ok(w) = self.world.read() {
            // Log robots
            for team in 0..=1 {
                for id in 0..num_robots {
                    let state = w.get_robot_state(id, team);
                    let (vx, vy, angular) = command_map
                        .get(&id)
                        .filter(|c| c.id == id && c.team == team)
                        .map(|c| (c.motion.vx, c.motion.vy, c.motion.angular))
                        .unwrap_or((0.0, 0.0, 0.0));

                    let _ = writeln!(
                        writer,
                        "{elapsed:.3},{id},{team},{vx},{vy},{angular},{},{},{},{},{}",
                        state.position.x,
                        state.position.y,
                        state.orientation,
                        state.velocity.x,
                        state.velocity.y
                    );
                }
            }

            // Log ball
            let ball = w.get_ball_state();
            let _ = writeln!(
                writer,
                "{elapsed:.3},-1,-1,0,0,0,{},{},0,{},{}",
                ball.position.x, ball.position.y, ball.velocity.x, ball.velocity.y
            );

            let _ = writer.flush();
            Some(())
        } else {
            None
        };

        if world.is_none() {
            warn!("Logger: failed to acquire world lock");
        }
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
