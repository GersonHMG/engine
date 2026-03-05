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
    pps: u32,
}

#[derive(serde::Serialize, Clone)]
struct RobotUpdate {
    id: u32,
    x: f64,
    y: f64,
    theta: f64,
    vx: f64,
    vy: f64,
    cmd_vx: f64,
    cmd_vy: f64,
    cmd_angular: f64,
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
        
        // Important for Windows/Localhost: Enable loopback
        socket.set_multicast_loop_v4(true)?;

        let addr = std::net::SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), port);
        socket.bind(&socket2::SockAddr::from(addr))?;
        
        // Join multicast group on default interface (0.0.0.0)
        // This usually works if routing table is correct or for simple setups.
        match socket.join_multicast_v4(&multicast_ip, &Ipv4Addr::new(0, 0, 0, 0)) {
            Ok(_) => info!("Joined multicast group on default interface"),
            Err(e) => warn!("Failed to join multicast on default interface: {e}"),
        }
        
        // Also try joining on Loopback interface (127.0.0.1) explicitly for local sims
        let _ = socket.join_multicast_v4(&multicast_ip, &Ipv4Addr::new(127, 0, 0, 1));

        let std_socket: std::net::UdpSocket = socket.into();
        UdpSocket::from_std(std_socket)?
    };

    info!(
        "Vision: Listening on port {port}, joined multicast group {multicast_addr} (Loopback enabled)"
    );

    let mut buf = vec![0u8; 65536];
    let mut tracker = Tracker::new();
    
    // PPS Calculation variables
    let mut packet_count: u32 = 0;
    let mut last_pps_calc = std::time::Instant::now();
    let mut current_pps: u32 = 0;

    loop {
        // Calculate PPS periodically (every 1s check? No, maybe just update on every 100 packets or keep a sliding count?)
        // Simplest: Check elapsed time every loop iteration? No, that's spammy.
        // Let's check elapsed time when we receive a packet.
        
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
                        
                        packet_count += 1;
                        let now = std::time::Instant::now();
                        if now.duration_since(last_pps_calc).as_secs() >= 1 {
                            current_pps = packet_count;
                            packet_count = 0;
                            last_pps_calc = now;
                        }

                        // Parse SSL_WrapperPacket
                        match crate::proto::protos::ssl_vision_wrapper::SSL_WrapperPacket::parse_from_bytes(data) {
                            Ok(wrapper) => {
                                if let Some(ref detection) = wrapper.detection.as_ref() {
                                    // Handle lock poisoning gracefully
                                    let mut world_writer = match world.write() {
                                        Ok(w) => w,
                                        Err(e) => {
                                            warn!("World lock poisoned, recovering");
                                            e.into_inner()
                                        }
                                    };

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
                                        pps: current_pps,
                                    };

                                    for robot in world_writer.blue_robots.values() {
                                         if robot.active {
                                             current_update.robots_blue.push(RobotUpdate {
                                                 id: robot.id as u32,
                                                 x: robot.position.x,
                                                 y: robot.position.y,
                                                 theta: robot.orientation,
                                                 vx: robot.velocity.x,
                                                 vy: robot.velocity.y,
                                                 cmd_vx: robot.commanded_velocity.x,
                                                 cmd_vy: robot.commanded_velocity.y,
                                                 cmd_angular: robot.commanded_angular,
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
                                                 vx: robot.velocity.x,
                                                 vy: robot.velocity.y,
                                                 cmd_vx: robot.commanded_velocity.x,
                                                 cmd_vy: robot.commanded_velocity.y,
                                                 cmd_angular: robot.commanded_angular,
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
