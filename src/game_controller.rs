// game_controller.rs — Multicast UDP receiver for SSL Game Controller referee messages
// Port of receivers/game_controller_ref.cpp + game_state.cpp

use prost::Message;
use std::net::Ipv4Addr;
use std::sync::{Arc, Mutex};
use tokio::net::UdpSocket;
use tracing::{info, warn};

/// Simple holder for the current referee command string.
pub struct GameState {
    ref_command: String,
}

impl GameState {
    pub fn new() -> Self {
        Self {
            ref_command: String::new(),
        }
    }

    pub fn get_ref_message(&self) -> &str {
        &self.ref_command
    }

    pub fn set_ref_command(&mut self, cmd: String) {
        self.ref_command = cmd;
    }
}

/// Convert a Referee::Command enum value to a string.
fn command_to_string(cmd: i32) -> &'static str {
    // These values match the Referee.Command enum in ssl_gc_referee_message.proto
    match cmd {
        0 => "HALT",
        1 => "STOP",
        2 => "NORMAL_START",
        3 => "FORCE_START",
        4 => "PREPARE_KICKOFF_YELLOW",
        5 => "PREPARE_KICKOFF_BLUE",
        6 => "PREPARE_PENALTY_YELLOW",
        7 => "PREPARE_PENALTY_BLUE",
        8 => "DIRECT_FREE_YELLOW",
        9 => "DIRECT_FREE_BLUE",
        12 => "TIMEOUT_YELLOW",
        13 => "TIMEOUT_BLUE",
        14 => "GOAL_YELLOW",
        15 => "GOAL_BLUE",
        16 => "BALL_PLACEMENT_YELLOW",
        17 => "BALL_PLACEMENT_BLUE",
        _ => "UNKNOWN",
    }
}

/// Run the Game Controller referee receiver loop.
pub async fn run_game_controller(
    multicast_addr: &str,
    port: u16,
    game_state: Arc<Mutex<GameState>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let multicast_ip: Ipv4Addr = multicast_addr.parse()?;

    let socket = {
        let socket = socket2::Socket::new(
            socket2::Domain::IPV4,
            socket2::Type::DGRAM,
            Some(socket2::Protocol::UDP),
        )?;
        socket.set_reuse_address(true)?;
        socket.set_nonblocking(true)?;

        let addr = std::net::SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port);
        socket.bind(&socket2::SockAddr::from(addr))?;
        socket.join_multicast_v4(&multicast_ip, &Ipv4Addr::UNSPECIFIED)?;

        let std_socket: std::net::UdpSocket = socket.into();
        UdpSocket::from_std(std_socket)?
    };

    info!(
        "GameController: Listening on port {port}, joined multicast group {multicast_addr}"
    );

    let mut buf = vec![0u8; 65536];

    loop {
        let (len, _src) = socket.recv_from(&mut buf).await?;
        let data = &buf[..len];

        match crate::proto::protos::Referee::decode(data) {
            Ok(ref_msg) => {
                let cmd_str = command_to_string(ref_msg.command as i32);
                if let Ok(mut gs) = game_state.lock() {
                    gs.set_ref_command(cmd_str.to_string());
                }
            }
            Err(e) => {
                warn!("Failed to parse referee message: {e}");
            }
        }
    }
}
