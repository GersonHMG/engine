// logger.rs — Unified Logging & Telemetry System
// Implements specs/testing_system.md V1.2.1

use crate::types::{BallState, RobotCommand, RobotState};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::time::Instant;
use std::sync::{Mutex, OnceLock, mpsc};
use tracing::{info, warn};

#[derive(Clone, Debug, PartialEq)]
pub enum LogValue {
    Float(f64),
    Str(String),
}

impl std::fmt::Display for LogValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogValue::Float(v) => write!(f, "{}", v),
            LogValue::Str(s) => write!(f, "{}", s),
        }
    }
}

#[derive(Debug)]
pub enum LoggerError {
    SessionNotActive,
    IoError(std::io::Error),
    ChannelError,
    #[allow(dead_code)]
    RegistryError(String),
}

impl std::fmt::Display for LoggerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoggerError::SessionNotActive => write!(f, "No active logging session"),
            LoggerError::IoError(e) => write!(f, "IO error: {}", e),
            LoggerError::ChannelError => write!(f, "Communication channel error"),
            LoggerError::RegistryError(s) => write!(f, "Registry error: {}", s),
        }
    }
}

impl std::error::Error for LoggerError {}

impl From<std::io::Error> for LoggerError {
    fn from(e: std::io::Error) -> Self {
        LoggerError::IoError(e)
    }
}

impl<T> From<std::sync::mpsc::SendError<T>> for LoggerError {
    fn from(_: std::sync::mpsc::SendError<T>) -> Self {
        LoggerError::ChannelError
    }
}

pub enum LogMessage {
    RegisterColumns {
        log_name: String,
        columns: Vec<String>,
    },
    LogCsv {
        log_name: String,
        elapsed_time: f64,
        is_main: bool,
        data: HashMap<String, LogValue>,
    },
    LogJson {
        log_name: String,
        data: serde_json::Value,
    },
    Stop,
}

pub struct LoggerRegistry {
    pub session_dir: Option<String>,
    pub session_start: Option<Instant>,
    pub current_elapsed: f64,
    pub columns: HashMap<String, Vec<String>>,
    pub pending_columns: HashMap<String, Vec<String>>,
    pub sender: Option<mpsc::Sender<LogMessage>>,
    pub bg_thread_handle: Option<std::thread::JoinHandle<()>>,
}

impl LoggerRegistry {
    pub fn new() -> Self {
        Self {
            session_dir: None,
            session_start: None,
            current_elapsed: 0.0,
            columns: HashMap::new(),
            pending_columns: HashMap::new(),
            sender: None,
            bg_thread_handle: None,
        }
    }

    pub fn start_session(&mut self, _log_name: &str) -> Result<(), LoggerError> {
        if self.session_dir.is_some() {
            self.stop_session();
        }

        let dir = get_session_dir_name();
        fs::create_dir_all(&dir)?;
        self.session_dir = Some(dir.clone());
        self.session_start = Some(Instant::now());
        self.current_elapsed = 0.0;

        let (tx, rx) = mpsc::channel::<LogMessage>();
        self.sender = Some(tx);

        let handle = std::thread::spawn(move || {
            run_file_writer_manager(dir, rx);
        });
        self.bg_thread_handle = Some(handle);

        info!("Logging session started in directory: {}", self.session_dir.as_ref().unwrap());

        // Register any pending columns that were defined before the session started
        let pending = std::mem::take(&mut self.pending_columns);
        for (name, cols) in pending {
            self.register_columns(&name, cols)?;
        }

        Ok(())
    }

    pub fn register_columns(&mut self, log_name: &str, columns: Vec<String>) -> Result<(), LoggerError> {
        let log_cols = self.columns.entry(log_name.to_string()).or_default();
        let mut new_cols = Vec::new();
        for col in columns {
            if !log_cols.contains(&col) {
                log_cols.push(col.clone());
                new_cols.push(col);
            }
        }

        if !new_cols.is_empty() {
            if let Some(ref sender) = self.sender {
                sender.send(LogMessage::RegisterColumns {
                    log_name: log_name.to_string(),
                    columns: log_cols.clone(),
                })?;
            }
        }
        Ok(())
    }

    pub fn stop_session(&mut self) {
        if let Some(sender) = self.sender.take() {
            let _ = sender.send(LogMessage::Stop);
        }
        if let Some(handle) = self.bg_thread_handle.take() {
            let _ = handle.join();
        }
        self.session_dir = None;
        self.session_start = None;
        self.current_elapsed = 0.0;
        self.columns.clear();
        self.pending_columns.clear();
        info!("Logging session stopped.");
    }

    pub fn set_current_elapsed(&mut self, elapsed: f64) {
        self.current_elapsed = elapsed;
    }

    pub fn session_start(&self) -> Option<Instant> {
        self.session_start
    }
}

pub static REGISTRY: OnceLock<Mutex<LoggerRegistry>> = OnceLock::new();

pub fn get_registry() -> &'static Mutex<LoggerRegistry> {
    REGISTRY.get_or_init(|| Mutex::new(LoggerRegistry::new()))
}

#[derive(Clone)]
pub struct Logger {
    log_name: String,
    #[allow(dead_code)]
    _columns: Vec<String>,
    is_main: bool,
}

impl Logger {
    /// Inicializa un logger dinámico a partir de un nombre de log.
    /// Genera internamente los archivos '{log_name}.csv' y '{log_name}.json'
    /// en la carpeta de la sesión actual.
    pub fn new(
        log_name: &str,
        columns: Vec<&str>,
        is_main: bool,
    ) -> Result<Self, LoggerError> {
        let mut reg = get_registry().lock().unwrap();

        if is_main && reg.session_dir.is_none() {
            reg.start_session(log_name)?;
        }

        let cols: Vec<String> = columns.into_iter().map(|s| s.to_string()).collect();
        if reg.session_dir.is_some() {
            reg.register_columns(log_name, cols.clone())?;
        } else {
            reg.pending_columns.entry(log_name.to_string()).or_default().extend(cols.clone());
        }

        Ok(Self {
            log_name: log_name.to_string(),
            _columns: cols,
            is_main,
        })
    }

    /// Registra valores en las columnas de este logger para el frame actual
    pub fn log_csv(&self, data: HashMap<String, LogValue>) -> Result<(), LoggerError> {
        let (elapsed_time, sender) = {
            let reg = get_registry().lock().unwrap();
            if reg.session_dir.is_none() {
                return Ok(()); // Silent no-op if session is not active
            }
            (reg.current_elapsed, reg.sender.clone())
        };

        if let Some(s) = sender {
            s.send(LogMessage::LogCsv {
                log_name: self.log_name.clone(),
                elapsed_time,
                is_main: self.is_main,
                data,
            })?;
        }
        Ok(())
    }

    /// Escribe metadatos en el reporte JSON general de la sesión
    pub fn log_json(&self, data: serde_json::Value) -> Result<(), LoggerError> {
        let sender = {
            let reg = get_registry().lock().unwrap();
            if reg.session_dir.is_none() {
                return Ok(()); // Silent no-op if session is not active
            }
            reg.sender.clone()
        };

        if let Some(s) = sender {
            s.send(LogMessage::LogJson {
                log_name: self.log_name.clone(),
                data,
            })?;
        }
        Ok(())
    }

    pub fn log_frame(
        &self,
        blue_robots: &[RobotState],
        yellow_robots: &[RobotState],
        ball: &BallState,
        command_map: &HashMap<(i32, i32), RobotCommand>,
    ) -> Result<(), LoggerError> {
        let num_robots = 6;
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
                    .get(&(id, team))
                    .map(|c| (c.motion.vx.unwrap_or(0.0), c.motion.vy.unwrap_or(0.0), c.motion.angular.unwrap_or(0.0)))
                    .unwrap_or((0.0, 0.0, 0.0));

                let mut data = HashMap::new();
                data.insert("RobotID".to_string(), LogValue::Float(id as f64));
                data.insert("Team".to_string(), LogValue::Float(team as f64));
                data.insert("Vx_Command".to_string(), LogValue::Float(vx_cmd));
                data.insert("Vy_Command".to_string(), LogValue::Float(vy_cmd));
                data.insert("Angular_Command".to_string(), LogValue::Float(angular_cmd));
                data.insert("Pos_X".to_string(), LogValue::Float(pos_x));
                data.insert("Pos_Y".to_string(), LogValue::Float(pos_y));
                data.insert("Orientation".to_string(), LogValue::Float(orientation));
                data.insert("Vx_Actual".to_string(), LogValue::Float(vx_actual));
                data.insert("Vy_Actual".to_string(), LogValue::Float(vy_actual));

                self.log_csv(data)?;
            }
        }

        let mut data = HashMap::new();
        data.insert("RobotID".to_string(), LogValue::Float(-1.0));
        data.insert("Team".to_string(), LogValue::Float(-1.0));
        data.insert("Vx_Command".to_string(), LogValue::Float(0.0));
        data.insert("Vy_Command".to_string(), LogValue::Float(0.0));
        data.insert("Angular_Command".to_string(), LogValue::Float(0.0));
        data.insert("Pos_X".to_string(), LogValue::Float(ball.position.x));
        data.insert("Pos_Y".to_string(), LogValue::Float(ball.position.y));
        data.insert("Orientation".to_string(), LogValue::Float(0.0));
        data.insert("Vx_Actual".to_string(), LogValue::Float(ball.velocity.x));
        data.insert("Vy_Actual".to_string(), LogValue::Float(ball.velocity.y));

        self.log_csv(data)?;

        Ok(())
    }
}

// --- Background File Writer Manager ---

struct LogFileState {
    file_path: String,
    columns: Vec<String>,
    rows: Vec<HashMap<String, LogValue>>,
    pending_ticks: HashMap<i64, (Vec<HashMap<String, LogValue>>, Vec<HashMap<String, LogValue>>)>,
}

fn run_file_writer_manager(session_dir: String, rx: mpsc::Receiver<LogMessage>) {
    let mut log_files: HashMap<String, LogFileState> = HashMap::new();

    while let Ok(msg) = rx.recv() {
        match msg {
            LogMessage::RegisterColumns { log_name, columns } => {
                let log_state = log_files.entry(log_name.clone()).or_insert_with(|| {
                    let file_path = format!("{}/{}.csv", session_dir, log_name);
                    LogFileState {
                        file_path,
                        columns: Vec::new(),
                        rows: Vec::new(),
                        pending_ticks: HashMap::new(),
                    }
                });

                let mut changed = false;
                for col in columns {
                    if !log_state.columns.contains(&col) {
                        log_state.columns.push(col);
                        changed = true;
                    }
                }

                if changed {
                    rewrite_csv_file(&log_state.file_path, &log_state.columns, &log_state.rows);
                }
            }
            LogMessage::LogCsv {
                log_name,
                elapsed_time,
                is_main,
                data,
            } => {
                let tick_ms = (elapsed_time * 1000.0).round() as i64;
                let log_state = log_files.entry(log_name.clone()).or_insert_with(|| {
                    let file_path = format!("{}/{}.csv", session_dir, log_name);
                    LogFileState {
                        file_path,
                        columns: Vec::new(),
                        rows: Vec::new(),
                        pending_ticks: HashMap::new(),
                    }
                });

                let (main_rows, sec_rows) = log_state.pending_ticks.entry(tick_ms).or_default();
                if is_main {
                    main_rows.push(data);
                } else {
                    sec_rows.push(data);
                }

                // Check for completed ticks: strictly older than current tick_ms
                let mut completed_ticks: Vec<i64> = log_state
                    .pending_ticks
                    .keys()
                    .filter(|&&t| t < tick_ms)
                    .cloned()
                    .collect();
                completed_ticks.sort();

                for t in completed_ticks {
                    if let Some((m_rows, s_rows)) = log_state.pending_ticks.remove(&t) {
                        let elapsed = (t as f64) / 1000.0;
                        let consolidated = consolidate_rows(elapsed, m_rows, s_rows);
                        log_state.rows.extend(consolidated.clone());
                        write_rows_to_file(&log_state.file_path, &log_state.columns, &consolidated);
                    }
                }
            }
            LogMessage::LogJson { log_name, data } => {
                let file_path = format!("{}/{}.json", session_dir, log_name);
                write_json_to_file(&file_path, data);
            }
            LogMessage::Stop => {
                // Flush all remaining pending ticks for all loggers
                for (_, mut log_state) in log_files.drain() {
                    let mut completed_ticks: Vec<i64> = log_state.pending_ticks.keys().cloned().collect();
                    completed_ticks.sort();
                    for t in completed_ticks {
                        if let Some((m_rows, s_rows)) = log_state.pending_ticks.remove(&t) {
                            let elapsed = (t as f64) / 1000.0;
                            let consolidated = consolidate_rows(elapsed, m_rows, s_rows);
                            log_state.rows.extend(consolidated.clone());
                            write_rows_to_file(&log_state.file_path, &log_state.columns, &consolidated);
                        }
                    }
                }
                break;
            }
        }
    }
}

fn consolidate_rows(
    elapsed_time: f64,
    main_rows: Vec<HashMap<String, LogValue>>,
    sec_rows: Vec<HashMap<String, LogValue>>,
) -> Vec<HashMap<String, LogValue>> {
    let mut final_rows = main_rows;

    for r in &mut final_rows {
        r.insert("ElapsedTime".to_string(), LogValue::Float(elapsed_time));
    }

    for mut s in sec_rows {
        if let (Some(LogValue::Float(robot_id)), Some(LogValue::Float(team))) = (s.get("RobotID"), s.get("Team")) {
            let matched = final_rows.iter_mut().find(|r| {
                r.get("RobotID").map(|id| {
                    if let LogValue::Float(id_f) = id {
                        (id_f - robot_id).abs() < 0.1
                    } else {
                        false
                    }
                }).unwrap_or(false)
                    && r.get("Team").map(|t| {
                        if let LogValue::Float(team_f) = t {
                            (team_f - team).abs() < 0.1
                        } else {
                            false
                        }
                    }).unwrap_or(false)
            });
            if let Some(r) = matched {
                for (k, v) in s {
                    if k != "RobotID" && k != "Team" {
                        r.insert(k, v);
                    }
                }
            } else {
                s.insert("ElapsedTime".to_string(), LogValue::Float(elapsed_time));
                final_rows.push(s);
            }
        } else {
            if let Some(first) = final_rows.get_mut(0) {
                for (k, v) in s {
                    first.insert(k, v);
                }
            } else {
                s.insert("ElapsedTime".to_string(), LogValue::Float(elapsed_time));
                final_rows.push(s);
            }
        }
    }

    final_rows
}

fn format_val(val: &LogValue, col_name: &str) -> String {
    match val {
        LogValue::Float(f) => {
            if col_name == "ElapsedTime" {
                format!("{:.3}", f)
            } else if col_name == "RobotID" || col_name == "Team" {
                format!("{:.0}", f)
            } else {
                format!("{}", f)
            }
        }
        LogValue::Str(s) => s.clone(),
    }
}

fn write_rows_to_file(file_path: &str, columns: &[String], rows: &[HashMap<String, LogValue>]) {
    let file_exists = std::path::Path::new(file_path).exists();
    let file = match fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_path)
    {
        Ok(f) => f,
        Err(e) => {
            warn!("Failed to open log file {} for appending: {}", file_path, e);
            return;
        }
    };

    let mut writer = BufWriter::new(file);

    if !file_exists && !columns.is_empty() {
        let header = columns.join(",");
        let _ = writeln!(writer, "{}", header);
    }

    for row in rows {
        let mut line_parts = Vec::new();
        for col in columns {
            if let Some(val) = row.get(col) {
                line_parts.push(format_val(val, col));
            } else {
                line_parts.push("".to_string());
            }
        }
        let _ = writeln!(writer, "{}", line_parts.join(","));
    }
    let _ = writer.flush();
}

fn rewrite_csv_file(file_path: &str, columns: &[String], rows: &[HashMap<String, LogValue>]) {
    let file = match fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(file_path)
    {
        Ok(f) => f,
        Err(e) => {
            warn!("Failed to open log file {} for rewriting: {}", file_path, e);
            return;
        }
    };

    let mut writer = BufWriter::new(file);

    if !columns.is_empty() {
        let header = columns.join(",");
        let _ = writeln!(writer, "{}", header);
    }

    for row in rows {
        let mut line_parts = Vec::new();
        for col in columns {
            if let Some(val) = row.get(col) {
                line_parts.push(format_val(val, col));
            } else {
                line_parts.push("".to_string());
            }
        }
        let _ = writeln!(writer, "{}", line_parts.join(","));
    }
    let _ = writer.flush();
}

fn write_json_to_file(file_path: &str, data: serde_json::Value) {
    let mut current_val = if std::path::Path::new(file_path).exists() {
        if let Ok(content) = fs::read_to_string(file_path) {
            serde_json::from_str(&content).unwrap_or(serde_json::Value::Null)
        } else {
            serde_json::Value::Null
        }
    } else {
        serde_json::Value::Null
    };

    match (&mut current_val, data) {
        (serde_json::Value::Object(current_obj), serde_json::Value::Object(new_obj)) => {
            for (k, v) in new_obj {
                current_obj.insert(k, v);
            }
        }
        (serde_json::Value::Array(current_arr), new_val) => {
            current_arr.push(new_val);
        }
        (curr, new_val) => {
            if curr.is_null() {
                *curr = new_val;
            } else {
                *curr = serde_json::Value::Array(vec![curr.clone(), new_val]);
            }
        }
    }

    if let Ok(file) = File::create(file_path) {
        let _ = serde_json::to_writer_pretty(file, &current_val);
    }
}

fn get_session_dir_name() -> String {
    let now = chrono::Local::now();
    now.format("logs/%Y-%m-%d_%H-%M-%S").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consolidate_rows_basic() {
        let main_rows = vec![
            {
                let mut m = HashMap::new();
                m.insert("RobotID".to_string(), LogValue::Float(0.0));
                m.insert("Team".to_string(), LogValue::Float(0.0));
                m.insert("Pos_X".to_string(), LogValue::Float(1.0));
                m
            },
            {
                let mut m = HashMap::new();
                m.insert("RobotID".to_string(), LogValue::Float(1.0));
                m.insert("Team".to_string(), LogValue::Float(0.0));
                m.insert("Pos_X".to_string(), LogValue::Float(2.0));
                m
            }
        ];

        let sec_rows = vec![
            {
                let mut m = HashMap::new();
                m.insert("RobotID".to_string(), LogValue::Float(0.0));
                m.insert("Team".to_string(), LogValue::Float(0.0));
                m.insert("target_theta".to_string(), LogValue::Float(3.14));
                m
            }
        ];

        let result = consolidate_rows(0.016, main_rows, sec_rows);
        assert_eq!(result.len(), 2);
        
        // Find row for robot 0
        let r0 = result.iter().find(|r| r.get("RobotID") == Some(&LogValue::Float(0.0))).unwrap();
        assert_eq!(r0.get("Pos_X"), Some(&LogValue::Float(1.0)));
        assert_eq!(r0.get("target_theta"), Some(&LogValue::Float(3.14)));
        assert_eq!(r0.get("ElapsedTime"), Some(&LogValue::Float(0.016)));

        // Find row for robot 1 (should have empty/None for target_theta)
        let r1 = result.iter().find(|r| r.get("RobotID") == Some(&LogValue::Float(1.0))).unwrap();
        assert_eq!(r1.get("Pos_X"), Some(&LogValue::Float(2.0)));
        assert_eq!(r1.get("target_theta"), None);
        assert_eq!(r1.get("ElapsedTime"), Some(&LogValue::Float(0.016)));
    }

    #[test]
    fn test_consolidate_rows_no_match_keys() {
        let main_rows = vec![
            {
                let mut m = HashMap::new();
                m.insert("RobotID".to_string(), LogValue::Float(0.0));
                m.insert("Team".to_string(), LogValue::Float(0.0));
                m.insert("Pos_X".to_string(), LogValue::Float(1.0));
                m
            }
        ];

        let sec_rows = vec![
            {
                let mut m = HashMap::new();
                m.insert("custom_var".to_string(), LogValue::Float(42.0));
                m
            }
        ];

        // Should merge into the first main row
        let result = consolidate_rows(0.016, main_rows, sec_rows);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].get("custom_var"), Some(&LogValue::Float(42.0)));
        assert_eq!(result[0].get("Pos_X"), Some(&LogValue::Float(1.0)));
    }

    #[test]
    fn test_idempotence_columns() {
        let mut reg = LoggerRegistry::new();
        // first registration
        reg.register_columns("test_log", vec!["col1".to_string(), "col2".to_string()]).unwrap();
        assert_eq!(reg.columns.get("test_log").unwrap().len(), 2);

        // second registration with overlapping columns
        reg.register_columns("test_log", vec!["col2".to_string(), "col3".to_string()]).unwrap();
        let cols = reg.columns.get("test_log").unwrap();
        assert_eq!(cols.len(), 3);
        assert_eq!(cols[0], "col1");
        assert_eq!(cols[1], "col2");
        assert_eq!(cols[2], "col3");
    }

    #[test]
    fn test_lua_logger_bridge() {
        let lua = mlua::Lua::new();
        crate::lua_interface::api::logger::register_logger_functions(&lua);

        let mut reg = get_registry().lock().unwrap();
        reg.start_session("system_log").unwrap();
        reg.register_columns("system_log", vec!["ElapsedTime".into(), "RobotID".into(), "Team".into()]).unwrap();
        drop(reg);

        let script = r#"
            local pid_logger = Logger.new("system_log", {"target_theta", "theta_error", "RobotRole"}, false)
            pid_logger:log_csv({
                RobotID = 0,
                Team = 0,
                target_theta = 3.14,
                theta_error = 0.5,
                RobotRole = "offense"
            })
            local str_logger = Logger.new("system_log", "SingleCol", false)
            str_logger:log_csv({
                RobotID = 0,
                Team = 0,
                SingleCol = 99.9
            })
            pid_logger:log_json({
                meta = "test_metadata",
                run = 123
            })
        "#;
        lua.load(script).exec().unwrap();

        let session_dir = {
            let mut reg = get_registry().lock().unwrap();
            let dir = reg.session_dir.clone();
            reg.stop_session();
            dir
        }.unwrap();

        let csv_path = format!("{}/system_log.csv", session_dir);
        let json_path = format!("{}/system_log.json", session_dir);

        assert!(std::path::Path::new(&csv_path).exists());
        assert!(std::path::Path::new(&json_path).exists());

        let csv_content = std::fs::read_to_string(&csv_path).unwrap();
        assert!(csv_content.contains("ElapsedTime,RobotID,Team,target_theta,theta_error,RobotRole,SingleCol"));
        assert!(csv_content.contains("0.000,0,0,3.14,0.5,offense,99.9"));

        let json_content = std::fs::read_to_string(&json_path).unwrap();
        assert!(json_content.contains("test_metadata"));

        let _ = std::fs::remove_dir_all(session_dir);
    }
}
