// pid.rs ΓÇö PID controller
// Port of motion/pid/pid.cpp

/// Simple PID controller.
pub struct PID {
    kp: f64,
    ki: f64,
    kd: f64,
    prev_error: f64,
    integral: f64,
}

impl PID {
    pub fn new(kp: f64, ki: f64, kd: f64) -> Self {
        Self {
            kp,
            ki,
            kd,
            prev_error: 0.0,
            integral: 0.0,
        }
    }

    pub fn compute(&mut self, error: f64, delta_time: f64) -> f64 {
        self.integral += error * delta_time;
        let derivative = (error - self.prev_error) / delta_time;
        let output = (self.kp * error) + (self.ki * self.integral) + (self.kd * derivative);
        self.prev_error = error;
        output
    }

    pub fn reset(&mut self) {
        self.prev_error = 0.0;
        self.integral = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pid_proportional() {
        let mut pid = PID::new(1.0, 0.0, 0.0);
        let output = pid.compute(5.0, 0.016);
        assert!((output - 5.0).abs() < 1e-9, "Pure P should return Kp * error");
    }

    #[test]
    fn pid_integral_accumulates() {
        let mut pid = PID::new(0.0, 1.0, 0.0);
        let o1 = pid.compute(1.0, 1.0);
        let o2 = pid.compute(1.0, 1.0);
        assert!((o1 - 1.0).abs() < 1e-9);
        assert!((o2 - 2.0).abs() < 1e-9);
    }

    #[test]
    fn pid_derivative() {
        let mut pid = PID::new(0.0, 0.0, 1.0);
        let _ = pid.compute(0.0, 1.0);
        let o2 = pid.compute(1.0, 1.0);
        assert!((o2 - 1.0).abs() < 1e-9);
    }
}
