// grsim.rs — grSim UDP command sender
// Port of radio/grsim.cpp

use protobuf::Message;
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
        let mut command = crate::proto::protos::grSim_Commands::GrSim_Robot_Command::new();
        command.set_id(id as u32);
        command.set_velangular(vel_angular as f32);
        command.set_kickspeedx(kick_speed_x as f32);
        command.set_kickspeedz(kick_speed_z as f32);
        command.set_veltangent(vel_x as f32);
        command.set_velnormal(vel_y as f32);
        command.set_spinner(spinner != 0);
        command.set_wheelsspeed(wheels_speed);

        let mut commands = crate::proto::protos::grSim_Commands::GrSim_Commands::new();
        commands.set_timestamp(0.0);
        commands.set_isteamyellow(team != 0);
        commands.robot_commands.push(command);

        let mut packet = crate::proto::protos::grSim_Packet::GrSim_Packet::new();
        packet.commands = protobuf::MessageField::some(commands);

        let encoded: Vec<u8> = packet.write_to_bytes().unwrap_or_default();
        if !encoded.is_empty() {
            let addr = format!("127.0.0.1:{GRSIM_COMMAND_PORT}");
            if let Err(e) = self.socket.send_to(&encoded, &addr) {
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

        let mut robot = crate::proto::protos::grSim_Replacement::GrSim_RobotReplacement::new();
        robot.set_x(x);
        robot.set_y(y);
        robot.set_dir(dir_deg);
        robot.set_id(id as u32);
        robot.set_yellowteam(yellow_team != 0);
        robot.turnon = Some(true);

        let mut replacement = crate::proto::protos::grSim_Replacement::GrSim_Replacement::new();
        replacement.robots.push(robot);

        let mut packet = crate::proto::protos::grSim_Packet::GrSim_Packet::new();
        packet.replacement = protobuf::MessageField::some(replacement);

        let encoded: Vec<u8> = packet.write_to_bytes().unwrap_or_default();
        if !encoded.is_empty() {
            let addr = format!("127.0.0.1:{GRSIM_COMMAND_PORT}");
            if let Err(e) = self.socket.send_to(&encoded, &addr) {
                warn!("grSim teleport send error: {e}");
            }
        }
    }

    /// Teleport the ball in grSim.
    pub fn communicate_pos_ball(&self, x: f64, y: f64) {
        let mut ball = crate::proto::protos::grSim_Replacement::GrSim_BallReplacement::new();
        ball.x = Some(x);
        ball.y = Some(y);
        ball.vx = Some(0.0);
        ball.vy = Some(0.0);

        let mut replacement = crate::proto::protos::grSim_Replacement::GrSim_Replacement::new();
        replacement.ball = protobuf::MessageField::some(ball);

        let mut packet = crate::proto::protos::grSim_Packet::GrSim_Packet::new();
        packet.replacement = protobuf::MessageField::some(replacement);

        let encoded: Vec<u8> = packet.write_to_bytes().unwrap_or_default();
        if !encoded.is_empty() {
            let addr = format!("127.0.0.1:{GRSIM_COMMAND_PORT}");
            if let Err(e) = self.socket.send_to(&encoded, &addr) {
                warn!("grSim ball teleport send error: {e}");
            }
        }
    }
}
