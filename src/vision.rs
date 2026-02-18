// vision.rs — Multicast UDP receiver for SSL Vision data

use crate::tracker::Tracker;
use crate::types::Vec2D;
use crate::world::World;
use prost::Message;
use std::net::Ipv4Addr;
use std::sync::{Arc, RwLock};
use tokio::net::UdpSocket;
use tracing::{info, warn};

/// Run the vision receiver loop. Listens on multicast UDP, parses SSL vision
/// packets, applies Kalman tracking, and updates the shared World state.
pub async fn run_vision(
    multicast_addr: &str,
    port: u16,
    world: Arc<RwLock<World>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let multicast_ip: Ipv4Addr = multicast_addr.parse()?;

    // Use socket2 to set up multicast before converting to tokio
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
        "Vision: Listening on port {port}, joined multicast group {multicast_addr}"
    );

    let mut buf = vec![0u8; 65536];
    let mut tracker = Tracker::new();

    loop {
        let (len, _src) = socket.recv_from(&mut buf).await?;
        let data = &buf[..len];

        // Parse SSL_WrapperPacket
        match crate::proto::protos::SslWrapperPacket::decode(data) {
            Ok(wrapper) => {
                if let Some(detection) = wrapper.detection {
                    // Process balls
                    for ball in &detection.balls {
                        let x = (ball.x / 1000.0) as f64;
                        let y = (ball.y / 1000.0) as f64;
                        let (_, _, _, vx, vy, _) = tracker.track(-1, -1, x, y, 0.0, 0.016);
                        if let Ok(mut w) = world.write() {
                            w.update_ball(Vec2D::new(vx, vy), Vec2D::new(x, y));
                        }
                    }

                    // Process blue robots
                    for robot in &detection.robots_blue {
                        process_robot(&mut tracker, &world, robot, 0);
                    }

                    // Process yellow robots
                    for robot in &detection.robots_yellow {
                        process_robot(&mut tracker, &world, robot, 1);
                    }
                }
            }
            Err(e) => {
                warn!("Failed to parse vision packet: {e}");
            }
        }
    }
}

fn process_robot(
    tracker: &mut Tracker,
    world: &Arc<RwLock<World>>,
    robot: &crate::proto::protos::SslDetectionRobot,
    team: i32,
) {
    let id = robot.robot_id.unwrap_or(0) as i32;
    let x = (robot.x / 1000.0) as f64;
    let y = (robot.y / 1000.0) as f64;
    let theta = robot.orientation.unwrap_or(0.0) as f64;
    let (_xf, _yf, _thetaf, vx, vy, omega) = tracker.track(team, id, x, y, theta, 0.016);

    if let Ok(mut w) = world.write() {
        w.update_robot(
            id,
            team,
            Vec2D::new(x, y),
            theta as f32,
            Vec2D::new(vx, vy),
            omega as f32,
        );
    }
}
