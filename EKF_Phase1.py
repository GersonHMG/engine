import numpy as np

def normalize_angle(angle):
    """Normalize angle to [-pi, pi]."""
    return (angle + np.pi) % (2 * np.pi) - np.pi

class ExtendedKalmanFilter:
    def __init__(
        self,
        initial_state,
        initial_cov,
        process_noise,
        measurement_noise,
    ):
        self.x = np.asarray(initial_state, dtype=float).copy()      # (7,)
        self.p = np.asarray(initial_cov, dtype=float).copy()        # (7,7)
        self.q_base = np.asarray(process_noise, dtype=float).copy()  # (7,7)
        self.r_base = np.asarray(measurement_noise, dtype=float).copy()  # (3,3)
        self.q = self.q_base.copy()
        self.r = self.r_base.copy()
        self.transient_counter = 0

        # Phase 1 model parameters
        self.a_threshold = 20.0
        self.decay_steps = 5
        self.alpha = 1000.0
        self.beta = 0.5

    def set_adaptive_parameters(self, a_threshold=20.0, decay_steps=5, alpha=1000.0, beta=0.5):
        self.a_threshold = a_threshold
        self.decay_steps = decay_steps
        self.alpha = alpha
        self.beta = beta

    def predict(self, dt):
        f_jac = self.jacobian_f(dt)
        self.x = self.f(dt)
        self.p = f_jac @ self.p @ f_jac.T + self.q

    def update(self, z):
        z = np.asarray(z, dtype=float)

        h_jac = self.jacobian_h()

        innovation = z - self.h()
        innovation[2] = normalize_angle(innovation[2])

        s = h_jac @ self.p @ h_jac.T + self.r

        try:
            s_inv = np.linalg.inv(s)
        except np.linalg.LinAlgError:
            s_inv = np.eye(3)

        k = self.p @ h_jac.T @ s_inv

        self.x = self.x + k @ innovation
        self.p = (np.eye(7) - k @ h_jac) @ self.p

        return innovation

    def trigger_transient(self, steps):
        self.transient_counter = steps

    def adapt_matrices(self, velocity_magnitude, confidence=1.0):
        # Adaptation of Q by velocity transient / kick
        if self.transient_counter > 0:
            self.q[4, 4] = self.q_base[4, 4] * self.alpha
            self.q[5, 5] = self.q_base[5, 5] * self.alpha
            self.transient_counter -= 1
        else:
            self.q[4, 4] = self.q_base[4, 4]
            self.q[5, 5] = self.q_base[5, 5]

        # Adaptation of R by velocity and confidence
        r_speed_scale = 1.0 + self.beta * velocity_magnitude
        r_confidence_scale = 1.0 / max(confidence, 0.01)
        total_scale = r_speed_scale * r_confidence_scale

        self.r[0, 0] = self.r_base[0, 0] * total_scale
        self.r[1, 1] = self.r_base[1, 1] * total_scale
        self.r[2, 2] = self.r_base[2, 2] * total_scale

    def filter_pose(self, x_meas, y_meas, theta_meas, dt, confidence=1.0):
        # Calculate preliminary inst acceleration before prediction
        prev_x, prev_y = self.x[0], self.x[1]
        prev_vx, prev_vy = self.x[4], self.x[5]
        
        v_inst_x = (x_meas - prev_x) / dt if dt > 0.0 else 0.0
        v_inst_y = (y_meas - prev_y) / dt if dt > 0.0 else 0.0
        a_inst_x = (v_inst_x - prev_vx) / dt if dt > 0.0 else 0.0
        a_inst_y = (v_inst_y - prev_vy) / dt if dt > 0.0 else 0.0
        a_magnitude = np.sqrt(a_inst_x**2 + a_inst_y**2)

        # Trigger transient if acceleration is high
        if a_magnitude > self.a_threshold:
            self.trigger_transient(self.decay_steps)

        # Adapt matrices
        current_v_mag = np.sqrt(prev_vx**2 + prev_vy**2)
        self.adapt_matrices(current_v_mag, confidence)

        # Predict + update
        self.predict(dt)
        z = np.array([x_meas, y_meas, theta_meas], dtype=float)
        innovation = self.update(z)

        x_f = self.x[0]
        y_f = self.x[1]
        theta_f = np.arctan2(self.x[2], self.x[3])
        vx = self.x[4]
        vy = self.x[5]
        omega = self.x[6]

        p_trace = np.trace(self.p)
        q_trace = np.trace(self.q)
        r_trace = np.trace(self.r)

        return (
            x_f,
            y_f,
            theta_f,
            vx,
            vy,
            omega,
            innovation,
            p_trace,
            q_trace,
            r_trace,
        )

    def f(self, dt):
        sin_theta = self.x[2]
        cos_theta = self.x[3]
        vx = self.x[4]
        vy = self.x[5]
        omega = self.x[6]

        theta = np.arctan2(sin_theta, cos_theta)
        theta_new = theta + omega * dt

        new_x = self.x.copy()
        new_x[0] += vx * dt
        new_x[1] += vy * dt
        new_x[2] = np.sin(theta_new)
        new_x[3] = np.cos(theta_new)

        return new_x

    def h(self):
        theta = np.arctan2(self.x[2], self.x[3])
        return np.array([
            self.x[0],
            self.x[1],
            theta,
        ])

    def jacobian_f(self, dt):
        f = np.eye(7)

        f[0, 4] = dt
        f[1, 5] = dt

        sin_theta = self.x[2]
        cos_theta = self.x[3]
        omega = self.x[6]

        theta = np.arctan2(sin_theta, cos_theta)
        theta_new = theta + omega * dt

        f[2, 6] = dt * np.cos(theta_new)
        f[3, 6] = -dt * np.sin(theta_new)

        return f

    def jacobian_h(self):
        h = np.zeros((3, 7))

        h[0, 0] = 1.0
        h[1, 1] = 1.0

        sin_theta = self.x[2]
        cos_theta = self.x[3]

        denom = sin_theta**2 + cos_theta**2

        if abs(denom) > 1e-12:
            h[2, 2] = cos_theta / denom
            h[2, 3] = -sin_theta / denom

        return h

    def set_noise_parameters(self, p_noise_p, p_noise_v, m_noise):
        self.q_base[0, 0] = p_noise_p
        self.q_base[1, 1] = p_noise_p
        self.q_base[4, 4] = p_noise_v
        self.q_base[5, 5] = p_noise_v
        self.r_base[0, 0] = m_noise
        self.r_base[1, 1] = m_noise
        self.r_base[2, 2] = m_noise

        self.q = self.q_base.copy()
        self.r = self.r_base.copy()
