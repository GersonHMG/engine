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
        # Adapt 7-state inputs to 11-state augmented system [x, y, sin(theta), cos(theta), vx, vy, omega, c_vx, c_vy, c_omega, c_theta]
        initial_state = np.asarray(initial_state, dtype=float)
        if len(initial_state) == 7:
            self.x = np.concatenate([initial_state, [0.0, 0.0, 0.0, 0.0]])
        else:
            self.x = initial_state.copy()

        initial_cov = np.asarray(initial_cov, dtype=float)
        if initial_cov.shape == (7, 7):
            self.p = np.eye(11) * 0.1
            self.p[:7, :7] = initial_cov
        else:
            self.p = initial_cov.copy()

        process_noise = np.asarray(process_noise, dtype=float)
        if process_noise.shape == (7, 7):
            self.q = np.eye(11) * 0.01
            self.q[:7, :7] = process_noise
            self.q[7, 7] = 0.01      # c_vx
            self.q[8, 8] = 0.01      # c_vy
            self.q[9, 9] = 0.01      # c_omega
            self.q[10, 10] = 0.001   # c_theta
        else:
            self.q = process_noise.copy()

        self.r = np.asarray(measurement_noise, dtype=float).copy()  # (3,3)
        self.innovation_history = []

        # Phase 3 model parameters
        self.gamma = 1.0
        self.lr = 0.05
        self.gamma_min = 0.1
        self.gamma_max = 10.0
        self.history_size = 30
        self.min_history = 10

    def set_adaptive_parameters(self, gamma=1.0, lr=0.05, gamma_min=0.1, gamma_max=10.0, history_size=30, min_history=10):
        self.gamma = gamma
        self.lr = lr
        self.gamma_min = gamma_min
        self.gamma_max = gamma_max
        self.history_size = history_size
        self.min_history = min_history

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
        self.p = (np.eye(11) - k @ h_jac) @ self.p

        # --- Phase 3: Innovation Whitening (Online Gamma Optimization) ---
        self.innovation_history.append(innovation.copy())
        if len(self.innovation_history) > self.history_size:
            self.innovation_history.pop(0)

        if len(self.innovation_history) >= self.min_history:
            innovs = np.array(self.innovation_history)  # (N, 3)
            mean_innovs = np.mean(innovs, axis=0)
            zero_mean_innovs = innovs - mean_innovs

            num = np.sum(zero_mean_innovs[1:] * zero_mean_innovs[:-1], axis=0)
            den = np.sum(zero_mean_innovs**2, axis=0) + 1e-8
            r1 = num / den  # Lag-1 autocorrelation of innovations

            # Update gamma based on autocorrelation of position innovations
            self.gamma = np.clip(self.gamma + self.lr * np.mean(r1[:2]), self.gamma_min, self.gamma_max)

        return innovation

    def filter_pose(self, x_meas, y_meas, theta_meas, dt):
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

        c_vx = self.x[7]
        c_vy = self.x[8]
        c_omega = self.x[9]
        c_theta = self.x[10]

        theta = np.arctan2(sin_theta, cos_theta)
        theta_new = theta + (omega + c_theta) * dt

        decay = np.exp(-self.gamma * dt)

        new_x = self.x.copy()
        new_x[0] += (vx + 0.5 * c_vx * dt) * dt
        new_x[1] += (vy + 0.5 * c_vy * dt) * dt
        new_x[2] = np.sin(theta_new)
        new_x[3] = np.cos(theta_new)
        new_x[4] += c_vx * dt
        new_x[5] += c_vy * dt
        new_x[6] += c_omega * dt
        new_x[7] *= decay
        new_x[8] *= decay
        new_x[9] *= decay
        new_x[10] *= decay

        return new_x

    def h(self):
        theta = np.arctan2(self.x[2], self.x[3])
        return np.array([
            self.x[0],
            self.x[1],
            theta,
        ])

    def jacobian_f(self, dt):
        f = np.eye(11)

        f[0, 4] = dt
        f[0, 7] = 0.5 * dt**2
        f[1, 5] = dt
        f[1, 8] = 0.5 * dt**2

        sin_theta = self.x[2]
        cos_theta = self.x[3]
        omega = self.x[6]
        c_theta = self.x[10]

        theta = np.arctan2(sin_theta, cos_theta)
        theta_new = theta + (omega + c_theta) * dt

        f[2, 6] = dt * np.cos(theta_new)
        f[2, 10] = dt * np.cos(theta_new)
        f[3, 6] = -dt * np.sin(theta_new)
        f[3, 10] = -dt * np.sin(theta_new)

        f[4, 7] = dt
        f[5, 8] = dt
        f[6, 9] = dt

        decay = np.exp(-self.gamma * dt)
        f[7, 7] = decay
        f[8, 8] = decay
        f[9, 9] = decay
        f[10, 10] = decay

        return f

    def jacobian_h(self):
        h = np.zeros((3, 11))

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
        self.q[0, 0] = p_noise_p
        self.q[1, 1] = p_noise_p
        self.q[4, 4] = p_noise_v
        self.q[5, 5] = p_noise_v
        self.r[0, 0] = m_noise
        self.r[1, 1] = m_noise
        self.r[2, 2] = m_noise
