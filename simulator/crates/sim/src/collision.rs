//! Robot body vs ball, in the robot's local frame (x forward).
//!
//! The body is a circle of radius R cut flat at x = f (the front face), so
//! its boundary is the face segment x = f, |y| ≤ h plus the arc of the
//! circle with x ≤ f, where h = sqrt(R² − f²).

use crate::config::RobotConfig;
use crate::math::{Vec2, EPSILON};

pub(crate) struct Contact {
    /// Closest point of the body boundary to the ball center (local).
    pub point: Vec2,
    /// Unit normal pointing out of the body towards the ball (local).
    pub normal: Vec2,
    /// How far the ball overlaps the body, m (> 0).
    pub depth: f64,
    /// The contact is on the flat front face.
    pub on_front: bool,
}

/// Contact between the body and a ball centered at `ball` (local frame),
/// or `None` if they do not touch.
pub(crate) fn robot_ball(config: &RobotConfig, ball: Vec2, ball_radius: f64) -> Option<Contact> {
    let front = config.front_distance;
    let point = closest_boundary_point(config, ball);
    let inside = ball.x <= front && ball.length() <= config.radius;

    let offset = ball - point;
    let distance = offset.length();
    let depth = if inside {
        ball_radius + distance
    } else {
        ball_radius - distance
    };
    if depth <= 0.0 {
        return None;
    }

    let on_front = point.x >= front - EPSILON;
    let normal = match offset.normalized() {
        Some(direction) if inside => -direction,
        Some(direction) => direction,
        // Ball center exactly on the boundary: use the surface normal.
        None if on_front => Vec2::new(1.0, 0.0),
        None => point.normalized().unwrap_or(Vec2::new(1.0, 0.0)),
    };

    Some(Contact {
        point,
        normal,
        depth,
        on_front,
    })
}

/// Closest point of the body boundary to `p`: on the face segment, or on
/// the arc when that part of the arc exists (x ≤ f).
fn closest_boundary_point(config: &RobotConfig, p: Vec2) -> Vec2 {
    let half = config.front_half_width();
    let on_face = Vec2::new(config.front_distance, p.y.clamp(-half, half));

    match p.normalized() {
        Some(direction) => {
            let on_arc = direction * config.radius;
            let arc_exists = on_arc.x <= config.front_distance;
            if arc_exists && (p - on_arc).length() < (p - on_face).length() {
                on_arc
            } else {
                on_face
            }
        }
        None => on_face,
    }
}
