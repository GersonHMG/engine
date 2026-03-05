// radio.rs — Radio dispatcher (serial port + grSim)
// Port of radio/radio.cpp

use crate::grsim::Grsim;
use crate::packet_serializer;
use crate::types::{KickerCommand, MotionCommand, RobotCommand};
use std::collections::{HashMap, HashSet};
use tracing::{debug, warn};

pub struct Radio {
    command_map: HashMap<i32, RobotCommand>,
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
                    debug!("Radio: serial port opened on {port_name} at {baud_rate} baud");
                    Some(port)
                }
                Err(e) => {
                    warn!("Radio: failed to open serial port {port_name}: {e}");
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

        // Close existing port (dropped)
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
                    debug!("Radio: reconfigured and opened on {port_name} at {baud_rate} baud");
                    self.serial_port = Some(port);
                }
                Err(e) => {
                    warn!("Radio: failed to open serial port {port_name}: {e}");
                }
            }
        } else {
            debug!("Radio: disabled (using grSim)");
        }
    }

    /// Pre-populate the command map with zero-velocity defaults for all
    /// robots that were actively commanded in the previous frame.  Call
    /// this at the start of each iteration, before any `add_*` calls.
    pub fn prepare_frame(&mut self) {
        for &(id, team) in &self.active_robots {
            self.command_map
                .entry(id)
                .or_insert_with(|| RobotCommand::new(id, team));
        }
    }

    pub fn add_motion_command(&mut self, motion: MotionCommand) {
        let id = motion.id;
        self.active_robots.insert((id, motion.team));

        let entry = self
            .command_map
            .entry(id)
            .or_insert_with(|| RobotCommand::new(id, motion.team));

        // Override the default zero vector with the incoming values.
        entry.motion.vx = motion.vx;
        entry.motion.vy = motion.vy;
        entry.motion.angular = motion.angular;
    }

    pub fn add_kicker_command(&mut self, kicker: KickerCommand) {
        let id = kicker.id;
        let entry = self
            .command_map
            .entry(id)
            .or_insert_with(|| RobotCommand::new(id, kicker.team));

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

        // Inject current commands to World for GUI/Log before sending
        for cmd in self.command_map.values() {
            world.set_commanded_velocity(
                cmd.id,
                cmd.team,
                crate::types::Vec2D::new(cmd.motion.vx, cmd.motion.vy),
                cmd.motion.angular,
            );
        }

        if self.use_radio {
            let buffer = packet_serializer::serialize(&self.command_map, 6);
            if let Some(ref mut port) = self.serial_port {
                if let Err(e) = port.write_all(&buffer) {
                    warn!("Radio serial write error: {e}");
                }
                if let Err(e) = port.flush() {
                    warn!("Radio serial flush error: {e}");
                }
            }
        } else {
            for cmd in self.command_map.values() {
                let m = &cmd.motion;
                let k = &cmd.kicker;

                self.grsim.communicate_grsim(
                    cmd.id,
                    cmd.team,
                    m.angular,
                    if k.kick_x { 3.0 } else { 0.0 },
                    if k.kick_z { 3.0 } else { 0.0 },
                    m.vx,
                    m.vy,
                    k.dribbler as i32,
                    false,
                );
            }
        }

        // Deregister robots that were sent a pure-zero command (no active
        // input this frame) so we stop sending to them next iteration.
        self.active_robots.retain(|&(id, _team)| {
            if let Some(cmd) = self.command_map.get(&id) {
                let m = &cmd.motion;
                m.vx != 0.0 || m.vy != 0.0 || m.angular != 0.0
            } else {
                false
            }
        });

        self.command_map.clear();
    }

    pub fn teleport_robot(&self, id: i32, team: i32, x: f64, y: f64, orientation: f64) {
        self.grsim.communicate_pos_robot(id, team, x, y, orientation);
        debug!(
            "[Lua] Teleport robot ID:{id} Team:{team} to ({x}, {y})"
        );
    }

    pub fn teleport_ball(&self, x: f64, y: f64) {
        self.grsim.communicate_pos_ball(x, y);
        debug!("[Lua] Teleport ball to ({x}, {y})");
    }

    pub fn get_command_map(&self) -> &HashMap<i32, RobotCommand> {
        &self.command_map
    }
}
