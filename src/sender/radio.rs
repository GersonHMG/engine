// radio.rs — Radio dispatcher (serial port + grSim)
// Port of radio/radio.cpp

use crate::sender::grsim::Grsim;
use crate::sender::packet_serializer;
use crate::types::{KickerCommand, MotionCommand, RobotCommand};
use std::collections::{HashMap, HashSet};
use tracing::{debug, warn};

pub struct Radio {
    // Keyed by (id, team) tuple to prevent cross-team overwrites
    command_map: HashMap<(i32, i32), RobotCommand>,
    /// Robots that received a command last frame and need a zero-velocity
    /// default until explicitly overridden again.
    active_robots: HashSet<(i32, i32)>,
    use_radio: bool,
    port_name: String,
    baud_rate: u32,
    serial_port: Option<Box<dyn serialport::SerialPort>>,
    grsim: Grsim,
}

impl Radio {
    pub fn new(use_radio: bool, port_name: &str, baud_rate: u32) -> Self {
        let serial_port = if use_radio {
            match serialport::new(port_name, baud_rate)
                .data_bits(serialport::DataBits::Eight)
                .parity(serialport::Parity::None)
                .stop_bits(serialport::StopBits::One)
                .flow_control(serialport::FlowControl::None)
                .timeout(std::time::Duration::from_millis(100))
                .open()
            {
                Ok(port) => {
                    debug!("Radio: serial port opened on {} at {} baud", port_name, baud_rate);
                    Some(port)
                }
                Err(e) => {
                    warn!("Radio: failed to open serial port {}: {}", port_name, e);
                    None
                }
            }
        } else {
            None
        };

        Self {
            command_map: HashMap::new(),
            active_robots: HashSet::new(),
            use_radio,
            port_name: port_name.to_string(),
            baud_rate,
            serial_port,
            grsim: Grsim::new(),
        }
    }

    pub fn reconfigure(&mut self, use_radio: bool, port_name: &str, baud_rate: u32) {
        self.use_radio = use_radio;
        self.port_name = port_name.to_string();
        self.baud_rate = baud_rate;

        // Close existing port (dropped automatically)
        self.serial_port = None;

        if use_radio {
            match serialport::new(port_name, baud_rate)
                .data_bits(serialport::DataBits::Eight)
                .parity(serialport::Parity::None)
                .stop_bits(serialport::StopBits::One)
                .flow_control(serialport::FlowControl::None)
                .timeout(std::time::Duration::from_millis(100))
                .open()
            {
                Ok(port) => {
                    debug!("Radio: reconfigured and opened on {} at {} baud", port_name, baud_rate);
                    self.serial_port = Some(port);
                }
                Err(e) => {
                    warn!("Radio: failed to open serial port {}: {}", port_name, e);
                }
            }
        } else {
            debug!("Radio: disabled (using grSim)");
        }
    }

    pub fn prepare_frame(&mut self) {
        for &(id, team) in &self.active_robots {
            self.command_map
                .entry((id, team))
                .or_insert_with(|| RobotCommand::new(id, team));
        }
    }

    pub fn add_motion_command(&mut self, motion: MotionCommand) {
        let id = motion.id;
        let team = motion.team;
        self.active_robots.insert((id, team));

        let entry = self
            .command_map
            .entry((id, team))
            .or_insert_with(|| RobotCommand::new(id, team));

        if let Some(vx) = motion.vx {
            entry.motion.vx = Some(vx);
        }
        if let Some(vy) = motion.vy {
            entry.motion.vy = Some(vy);
        }
        if let Some(angular) = motion.angular {
            entry.motion.angular = Some(angular);
        }
    }

    pub fn add_kicker_command(&mut self, kicker: KickerCommand) {
        let id = kicker.id;
        let team = kicker.team;
        
        let entry = self
            .command_map
            .entry((id, team))
            .or_insert_with(|| RobotCommand::new(id, team));

        let existing = &mut entry.kicker;
        if kicker.kick_x {
            existing.kick_x = true;
        }
        if kicker.kick_z {
            existing.kick_z = true;
        }
        if kicker.dribbler != 0.0 {
            existing.dribbler = kicker.dribbler;
        }
    }

    pub fn send_commands(&mut self, world: &mut crate::world::World) {
        if self.command_map.is_empty() {
            return;
        }

        // 1. Inject current commands to World for GUI/Log before sending
        for cmd in self.command_map.values() {
            world.set_commanded_velocity(
                cmd.id,
                cmd.team,
                crate::types::Vec2D::new(cmd.motion.vx.unwrap_or(0.0), cmd.motion.vy.unwrap_or(0.0)),
                cmd.motion.angular.unwrap_or(0.0),
            );
        }

        // 2. HARDWARE RADIO LOGIC (Strict Serializer)
        if self.use_radio {
            // Group commands by team so Robot 0 (Blue) doesn't overwrite Robot 0 (Yellow)
            let mut commands_by_team: HashMap<i32, HashMap<i32, RobotCommand>> = HashMap::new();

            for (&(id, team), cmd) in &self.command_map {
                commands_by_team
                    .entry(team)
                    .or_default()
                    .insert(id, cmd.clone());
            }

            // Serialize and transmit a separate packet for each team
            for (team_id, single_team_map) in commands_by_team {
                // Pass the strict HashMap<i32, RobotCommand> to your firmware serializer
                let buffer = packet_serializer::serialize(&single_team_map, team_id as usize);
                
                if let Some(ref mut port) = self.serial_port {
                    if let Err(e) = port.write_all(&buffer) {
                        warn!("Radio serial write error (Team {}): {}", team_id, e);
                    }
                    if let Err(e) = port.flush() {
                        warn!("Radio serial flush error (Team {}): {}", team_id, e);
                    }
                }
            }
        } 
        // 3. SIMULATOR LOGIC (Supports tuple map directly)
        else {
            for cmd in self.command_map.values() {
                let m = &cmd.motion;
                let k = &cmd.kicker;

                self.grsim.communicate_grsim(
                    cmd.id,
                    cmd.team,
                    m.angular.unwrap_or(0.0),
                    if k.kick_x { 3.0 } else { 0.0 },
                    if k.kick_z { 3.0 } else { 0.0 },
                    m.vx.unwrap_or(0.0),
                    m.vy.unwrap_or(0.0),
                    k.dribbler as i32,
                    false,
                );
            }
        }

        // 4. Deregister stationary robots
        self.active_robots.retain(|&(id, team)| {
            if let Some(cmd) = self.command_map.get(&(id, team)) {
                let m = &cmd.motion;
                m.vx.unwrap_or(0.0) != 0.0 || m.vy.unwrap_or(0.0) != 0.0 || m.angular.unwrap_or(0.0) != 0.0
            } else {
                false
            }
        });

        self.command_map.clear();
    }

    pub fn teleport_robot(&self, id: i32, team: i32, x: f64, y: f64, orientation: f64) {
        self.grsim.communicate_pos_robot(id, team, x, y, orientation);
        debug!("[Lua] Teleport robot ID:{} Team:{} to ({}, {})", id, team, x, y);
    }

    pub fn teleport_ball(&self, x: f64, y: f64) {
        self.grsim.communicate_pos_ball(x, y);
        debug!("[Lua] Teleport ball to ({}, {})", x, y);
    }

    pub fn get_command_map(&self) -> &HashMap<(i32, i32), RobotCommand> {
        &self.command_map
    }
}