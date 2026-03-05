// lookahead.rs - LookAhead PID controller ported from python

use crate::types::Vec2D;
use std::f64::consts::PI;

pub struct LookAheadPID {
    kp_lat: f64,
    ki_lat: f64,
    kd_lat: f64,
    kp_speed: f64,
    kp_heading: f64,
    target_speed: f64,
    lookahead: f64,

    integral_lat: f64,
    prev_error_lat: f64,
    pub closest_idx: usize,
}

impl LookAheadPID {
    pub fn new(
        kp_lat: f64,
        ki_lat: f64,
        kd_lat: f64,
        kp_speed: f64,
        kp_heading: f64,
        target_speed: f64,
        lookahead: f64,
    ) -> Self {
        Self {
            kp_lat,
            ki_lat,
            kd_lat,
            kp_speed,
            kp_heading,
            target_speed,
            lookahead,
            integral_lat: 0.0,
            prev_error_lat: 0.0,
            closest_idx: 0,
        }
    }

    pub fn reset(&mut self) {
        self.integral_lat = 0.0;
        self.prev_error_lat = 0.0;
        self.closest_idx = 0;
    }

    pub fn compute(&mut self, pos: Vec2D, theta: f64, vel: Vec2D, path: &[Vec2D], dt: f64) -> (f64, f64, f64) {
        if path.is_empty() {
            return (0.0, 0.0, 0.0);
        }

        let n = path.len();

        // 1) Find closest waypoint
        let search_start = if self.closest_idx > 5 { self.closest_idx - 5 } else { 0 };
        let search_end = usize::min(n, self.closest_idx + 40);

        let mut min_dist = f64::MAX;
        let mut min_idx = search_start;

        for i in search_start..search_end {
            let p = path[i];
            let dist = (p - pos).length();
            if dist < min_dist {
                min_dist = dist;
                min_idx = i;
            }
        }
        self.closest_idx = min_idx;

        // 2) Look-ahead point
        let mut lookahead_idx = self.closest_idx;
        let mut accumulated = 0.0;
        while lookahead_idx < n - 1 && accumulated < self.lookahead {
            let seg = (path[lookahead_idx + 1] - path[lookahead_idx]).length();
            accumulated += seg;
            lookahead_idx += 1;
        }
        let target_pt = path[lookahead_idx];

        // 3) Vector to target 
        let dx = target_pt.x - pos.x;
        let dy = target_pt.y - pos.y;

        // 4) Local frame error
        let cos_t = theta.cos();
        let sin_t = theta.sin();

        // Note: Python was local x = forward, y = left
        let err_lateral = -dx * sin_t + dy * cos_t;

        // 5) Lateral PID
        if dt > 0.0 {
            self.integral_lat += err_lateral * dt;
            self.integral_lat = self.integral_lat.clamp(-2.0, 2.0);
        }
        
        let deriv_lat = if dt > 0.0 {
            (err_lateral - self.prev_error_lat) / dt
        } else {
            0.0
        };
        self.prev_error_lat = err_lateral;

        let vy_cmd = self.kp_lat * err_lateral + self.ki_lat * self.integral_lat + self.kd_lat * deriv_lat;

        // 6) Longitudinal speed
        let current_speed = vel.length();
        let speed_error = self.target_speed - current_speed;
        let mut vx_cmd = self.target_speed + self.kp_speed * speed_error;

        // Slow down at end of path
        let _remaining_points = n.saturating_sub(1).saturating_sub(self.closest_idx);
        let dist_to_end = (*path.last().unwrap() - pos).length();

        // If we are within 2.0 meters of the end, start scaling down velocity. Mute velocity fully at 0.1m.
        if dist_to_end < 2.0 {
            vx_cmd *= (dist_to_end / 2.0).max(0.0);
        }

        // 7) Heading 
        let desired_heading = dy.atan2(dx);
        let mut heading_error = desired_heading - theta;
        heading_error = (heading_error + PI) % (2.0 * PI);
        if heading_error < 0.0 {
            heading_error += 2.0 * PI;
        }
        heading_error -= PI;

        let omega_cmd = self.kp_heading * heading_error;

        // Output returning vx (forward), vy (left), omega
        (vx_cmd, vy_cmd, omega_cmd)
    }
}
