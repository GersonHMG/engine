// vision.rs — Multicast UDP receiver for SSL Vision data

use crate::tracker::Tracker;
use crate::types::Vec2D;
use crate::world::World;
use protobuf::Message;
use std::net::Ipv4Addr;
use std::sync::{Arc, RwLock};
use tokio::net::UdpSocket;

use tracing::{info, warn};
use tauri::Manager;

#[derive(serde::Serialize, Clone)]
struct VisionUpdate {
    ball: Option<Vec2D>,
    robots_blue: Vec<RobotUpdate>,
    robots_yellow: Vec<RobotUpdate>,
}

#[derive(serde::Serialize, Clone)]
struct RobotUpdate {
    id: u32,
    x: f64,
    y: f64,
    theta: f64,
}

// VisionCommand enum
#[derive(Debug, Clone)]
pub enum VisionCommand {
    UpdateTrackerConfig {
        enabled: bool,
        process_noise_p: f64,
        process_noise_v: f64,
        measurement_noise: f64,
    },
}

/// Run the vision receiver loop. Listens on multicast UDP, parses SSL vision
/// packets, applies Kalman tracking, and updates the shared World state.
pub async fn run_vision(
    multicast_addr: String,
    port: u16,
    world: Arc<RwLock<World>>,
    app_handle: tauri::AppHandle,
    mut command_rx: tokio::sync::mpsc::Receiver<VisionCommand>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
        tokio::select! {
            cmd = command_rx.recv() => {
                match cmd {
                    Some(VisionCommand::UpdateTrackerConfig { enabled, process_noise_p, process_noise_v, measurement_noise }) => {
                        info!("Updating tracker config: enabled={}, p_noise_p={}, p_noise_v={}, m_noise={}", enabled, process_noise_p, process_noise_v, measurement_noise);
                        tracker.update_config(enabled, process_noise_p, process_noise_v, measurement_noise);
                    }
                    None => {
                        info!("Vision command channel closed, stopping vision task.");
                        break;
                    }
                }
            }
            res = socket.recv_from(&mut buf) => {
                match res {
                    Ok((len, _src)) => {
                        let data = &buf[..len];

                        // Parse SSL_WrapperPacket
                        match crate::proto::protos::ssl_vision_wrapper::SSL_WrapperPacket::parse_from_bytes(data) {
                            Ok(wrapper) => {
                                if let Some(ref detection) = wrapper.detection.as_ref() {
                                    let mut world_writer = world.write().unwrap();

                                    // Process balls
                                    for ball in &detection.balls {
                                        let x = (ball.x() / 1000.0) as f64;
                                        let y = (ball.y() / 1000.0) as f64;
                                        let (_, _, _, vx, vy, _) = tracker.track(-1, -1, x, y, 0.0, 0.016);
                                        world_writer.update_ball(Vec2D::new(vx, vy), Vec2D::new(x, y));
                                    }

                                    // Process blue robots
                                    for robot in &detection.robots_blue {
                                        process_robot(&mut tracker, &mut world_writer, robot, 0);
                                    }

                                    // Process yellow robots
                                    for robot in &detection.robots_yellow {
                                        process_robot(&mut tracker, &mut world_writer, robot, 1);
                                    }
                                    
                                    // Create update struct
                                    let mut current_update = VisionUpdate {
                                        ball: Some(world_writer.ball.position),
                                        robots_blue: vec![],
                                        robots_yellow: vec![],
                                    };

                                    for robot in world_writer.blue_robots.values() {
                                         if robot.active {
                                             current_update.robots_blue.push(RobotUpdate {
                                                 id: robot.id as u32,
                                                 x: robot.position.x,
                                                 y: robot.position.y,
                                                 theta: robot.orientation,
                                             });
                                         }
                                    }

                                    for robot in world_writer.yellow_robots.values() {
                                         if robot.active {
                                             current_update.robots_yellow.push(RobotUpdate {
                                                 id: robot.id as u32,
                                                 x: robot.position.x,
                                                 y: robot.position.y,
                                                 theta: robot.orientation,
                                             });
                                         }
                                    }

                                    // Emit event
                                     let _ = app_handle.emit_all("vision-update", &current_update);
                                }
                            }
                            Err(e) => {
                                warn!("Failed to parse vision packet: {e}");
                            }
                        }
                    }
                    Err(e) => {
                         warn!("Vision socket error: {e}");
                    }
                }
            }
        }
    }
    Ok(())
}

fn process_robot(
    tracker: &mut Tracker,
    world: &mut World,
    robot: &crate::proto::protos::ssl_vision_detection::SSL_DetectionRobot,
    team: i32,
) {
    let id = robot.robot_id() as i32;
    let x = (robot.x() / 1000.0) as f64;
    let y = (robot.y() / 1000.0) as f64;
    let theta = robot.orientation() as f64;
    let (_xf, _yf, _thetaf, vx, vy, omega) = tracker.track(team, id, x, y, theta, 0.016);

    world.update_robot(
        id,
        team,
        Vec2D::new(x, y),
        theta as f32,
        Vec2D::new(vx, vy),
        omega as f32,
    );
}
