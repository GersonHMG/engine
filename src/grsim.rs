// grsim.rs — grSim UDP command sender
// Port of radio/grsim.cpp

use prost::Message;
use std::net::UdpSocket;
use tracing::warn;

const GRSIM_COMMAND_PORT: u16 = 20011;

pub struct Grsim {
    socket: UdpSocket,
}

impl Grsim {
    pub fn new() -> Self {
        let socket = UdpSocket::bind("0.0.0.0:0").expect("Failed to bind grSim UDP socket");
        Self { socket }
    }

    /// Send a motion command to grSim.
    pub fn communicate_grsim(
        &self,
        id: i32,
        team: i32,
        vel_angular: f64,
        kick_speed_x: f64,
        kick_speed_z: f64,
        vel_x: f64,
        vel_y: f64,
        spinner: i32,
        wheels_speed: bool,
    ) {
        let command = crate::proto::protos::GrSimRobotCommand {
            id: id as u32,
            velangular: vel_angular as f32,
            kickspeedx: kick_speed_x as f32,
            kickspeedz: kick_speed_z as f32,
            veltangent: vel_x as f32,
            velnormal: vel_y as f32,
            spinner: spinner != 0,
            wheelsspeed: wheels_speed,
            wheel1: None,
            wheel2: None,
            wheel3: None,
            wheel4: None,
        };

        let commands = crate::proto::protos::GrSimCommands {
            timestamp: 0.0,
            isteamyellow: team != 0,
            robot_commands: vec![command],
        };

        let packet = crate::proto::protos::GrSimPacket {
            commands: Some(commands),
            replacement: None,
        };

        let mut buf = Vec::new();
        if packet.encode(&mut buf).is_ok() && !buf.is_empty() {
            let addr = format!("127.0.0.1:{GRSIM_COMMAND_PORT}");
            if let Err(e) = self.socket.send_to(&buf, &addr) {
                warn!("grSim send error: {e}");
            }
        }
    }

    /// Teleport a robot in grSim.
    pub fn communicate_pos_robot(
        &self,
        id: i32,
        yellow_team: i32,
        x: f64,
        y: f64,
        dir: f64,
    ) {
        let dir_deg = dir.to_degrees();

        let robot = crate::proto::protos::GrSimRobotReplacement {
            x,
            y,
            dir: dir_deg,
            id: id as u32,
            yellowteam: yellow_team != 0,
            turnon: Some(true),
        };

        let replacement = crate::proto::protos::GrSimReplacement {
            robots: vec![robot],
            ball: None,
        };

        let packet = crate::proto::protos::GrSimPacket {
            commands: None,
            replacement: Some(replacement),
        };

        let mut buf = Vec::new();
        if packet.encode(&mut buf).is_ok() && !buf.is_empty() {
            let addr = format!("127.0.0.1:{GRSIM_COMMAND_PORT}");
            if let Err(e) = self.socket.send_to(&buf, &addr) {
                warn!("grSim teleport send error: {e}");
            }
        }
    }

    /// Teleport the ball in grSim.
    pub fn communicate_pos_ball(&self, x: f64, y: f64) {
        let ball = crate::proto::protos::GrSimBallReplacement {
            x: Some(x),
            y: Some(y),
            vx: Some(0.0),
            vy: Some(0.0),
        };

        let replacement = crate::proto::protos::GrSimReplacement {
            ball: Some(ball),
            robots: vec![],
        };

        let packet = crate::proto::protos::GrSimPacket {
            commands: None,
            replacement: Some(replacement),
        };

        let mut buf = Vec::new();
        if packet.encode(&mut buf).is_ok() && !buf.is_empty() {
            let addr = format!("127.0.0.1:{GRSIM_COMMAND_PORT}");
            if let Err(e) = self.socket.send_to(&buf, &addr) {
                warn!("grSim ball teleport send error: {e}");
            }
        }
    }
}
