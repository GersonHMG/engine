# Especificación Técnica: Filtro de Kalman Adaptativo (AKF) en SYSMIC

Este documento describe la especificación técnica y el plan de implementación para transformar el Filtro de Kalman Extendido (EKF) estático en un **Filtro de Kalman Adaptativo (AKF)** en el módulo de tracking de SYSMIC, localizado en [engine/src/receiver/tracker.rs](file:///home/mrepetto/Documentos/sysmic/engine/src/receiver/tracker.rs).

---

## 1. Justificación y Objetivos

### Problema Actual
El tracker actual utiliza matrices de ruido de proceso ($Q$) y ruido de medición ($R$) con valores fijos y estáticos. Esto causa:
1.  **Retraso de Convergencia (Lag) en Disparos:** Al patear la pelota, la aceleración instantánea (un escalón de velocidad) tarda de 8 a 10 tramas en ser asimilada por el EKF. Las predicciones del punto de parada de la pelota sufren picos de desviación de hasta 4 metros.
2.  **Inestabilidad Numérica sin Filtrado:** Intentar desactivar o reducir drásticamente el filtrado para mitigar el lag produce oscilaciones extremas de velocidad debido a la discretización en milímetros y el jitter de red de **SSL Vision**, congelando los robots en su posición.

### Objetivo
Diseñar una sintonización dinámica en caliente de las matrices $Q$ y $R$ para:
*   Reducir el retraso de convergencia de velocidad de la pelota tras un impacto a **menos de 2 frames**.
*   Mantener la atenuación del ruido (jitter) a bajas velocidades para conservar la estabilidad del control PID de movimiento.

---

## 2. Formulación Matemática de la Adaptabilidad

El Filtro de Kalman predice la covarianza del error del estado mediante:
$$P_k^- = F_k P_{k-1} F_k^T + Q_k$$

Y calcula la ganancia de Kalman $K_k$ usando el ruido de medición $R_k$:
$$S_k = H_k P_k^- H_k^T + R_k$$
$$K_k = P_k^- H_k^T S_k^{-1}$$

En el **Kalman Adaptativo (AKF)**, ajustamos $Q_k$ y $R_k$ en cada paso temporal.

### A. Ruido de Proceso Adaptativo ($Q_k$) para Mitigación de Lag
El ruido de proceso representa la incertidumbre del modelo físico (velocidad constante). Cuando ocurre un impacto o un cambio brusco de velocidad, el modelo físico de velocidad constante deja de ser válido, por lo que debemos **incrementar temporalmente** la incertidumbre del proceso ($Q$) para que el filtro confíe más en la medición nueva y menos en su predicción previa.

#### Algoritmo de Detección de Transitorios de la Pelota
1.  Calcular la aceleración instantánea entre la medición cruda actual $z_k = [x_m, y_m]^T$ y el estado estimado anterior $x_{k-1} = [x, y, vx, vy]^T$:
    $$\vec{v}_{inst} = \frac{\vec{p}_m - \vec{p}_{k-1}}{dt}$$
    $$\vec{a}_{inst} = \frac{\vec{v}_{inst} - \vec{v}_{k-1}}{dt}$$
2.  Si la magnitud de la aceleración $\|\vec{a}_{inst}\|$ supera un umbral crítico $a_{\text{kick}}$ (ej: $15.0\text{ m/s}^2$):
    *   Activar el **Modo Transitorio de Velocidad**.
    *   Establecer un contador de decaimiento: $N_{\text{decay}} = 5$ frames.
3.  Mientras $N_{\text{decay}} > 0$:
    *   Escalar los componentes de velocidad de la matriz de ruido de proceso $Q$ mediante un factor multiplicador masivo:
        $$Q_k(vx, vx) = Q_0(vx, vx) \cdot \alpha$$
        $$Q_k(vy, vy) = Q_0(vy, vy) \cdot \alpha$$
        Donde $\alpha \approx 10^3$ o $10^4$ (sintonizable).
    *   Decrementar $N_{\text{decay}}$ en cada tick.
4.  Al llegar a $N_{\text{decay}} = 0$, restaurar $Q_k$ a su valor basal $Q_0$.

### B. Ruido de Medición Adaptativo ($R_k$) para Robustez contra Jitter y Motion Blur
El ruido de medición representa la incertidumbre de la cámara. A alta velocidad, la pelota sufre de *motion blur* y oclusiones frecuentes. A baja velocidad, queremos filtrar al máximo el jitter.

1.  **Ajuste por Velocidad (Pelota):**
    Establecer $R_k$ proporcional a la velocidad estimada del objeto:
    $$R_k = R_0 \cdot \left(1.0 + \beta \cdot \|\vec{v}_{k-1}\|\right)$$
    Donde $\beta$ es un parámetro que incrementa la tolerancia al ruido a alta velocidad (evitando saltos bruscos por malas lecturas).
2.  **Ajuste por Pérdida de Confianza de Visión:**
    Integrar el estimador de confianza de ZJUNlict ($P_{o, t} \in [0, 1]$). Si el objeto es visto constantemente, $P_{o, t} \to 1.0$ y usamos $R_0$. Si el objeto tiene baja visibilidad (oclusión parcial), $P_{o, t} \to 0$ y aumentamos $R_k$ para depender casi enteramente de la predicción cinemática:
    $$R_k = R_0 \cdot \frac{1}{\max(P_{o, t}, \epsilon)}$$

---

## 3. Arquitectura del Código a Implementar

Se proponen modificaciones en el tracker de [engine/src/receiver/tracker.rs](file:///home/mrepetto/Documentos/sysmic/engine/src/receiver/tracker.rs):

### A. Modificaciones en `ExtendedKalmanFilter` en [engine/src/receiver/tracker.rs](file:///home/mrepetto/Documentos/sysmic/engine/src/receiver/tracker.rs)

1.  **Campos Nuevos en la Estructura:**
    ```rust
    pub struct ExtendedKalmanFilter {
        pub x: SVector<f64, 7>,          // State
        pub p: SMatrix<f64, 7, 7>,       // Covariance
        pub q_base: SMatrix<f64, 7, 7>,  // Almacenar el q original
        pub r_base: SMatrix<f64, 3, 3>,  // Almacenar el r original
        pub q: SMatrix<f64, 7, 7>,       // Process noise adaptado
        pub r: SMatrix<f64, 3, 3>,       // Measurement noise adaptado
        pub transient_counter: usize,    // N_decay
    }
    ```
2.  **Método de Adaptabilidad:**
    ```rust
    impl ExtendedKalmanFilter {
        /// Activa el modo transitorio incrementando temporalmente Q
        pub fn trigger_transient(&mut self, steps: usize) {
            self.transient_counter = steps;
        }

        /// Actualiza dinámicamente Q y R antes del paso de predicción/update
        pub fn adapt_matrices(&mut self, velocity_magnitude: f64, confidence: f64) {
            // Adaptación de q por transitorio de velocidad/kicking
            if self.transient_counter > 0 {
                let alpha = 1000.0;
                self.q[(4, 4)] = self.q_base[(4, 4)] * alpha; // vx
                self.q[(5, 5)] = self.q_base[(5, 5)] * alpha; // vy
                self.transient_counter -= 1;
            } else {
                self.q[(4, 4)] = self.q_base[(4, 4)];
                self.q[(5, 5)] = self.q_base[(5, 5)];
            }

            // Adaptación de r por velocidad y confianza
            let beta = 0.5;
            let r_speed_scale = 1.0 + beta * velocity_magnitude;
            let r_confidence_scale = 1.0 / confidence.max(0.01);
            
            let total_scale = r_speed_scale * r_confidence_scale;
            self.r[(0, 0)] = self.r_base[(0, 0)] * total_scale;
            self.r[(1, 1)] = self.r_base[(1, 1)] * total_scale;
            self.r[(2, 2)] = self.r_base[(2, 2)] * total_scale;
        }
    }
    ```

### B. Modificaciones en `Tracker` en [engine/src/receiver/tracker.rs](file:///home/mrepetto/Documentos/sysmic/engine/src/receiver/tracker.rs)

1.  **Detección de Aceleraciones en `track()`:**
    ```rust
    pub fn track(
        &mut self,
        team: i32,
        id: i32,
        x: f64,
        y: f64,
        theta: f64,
        dt: f64,
    ) -> (f64, f64, f64, f64, f64, f64) {
        // ... (código existente para recuperar/crear el filtro en tracker.rs) ...

        let filter = self
            .filters
            .entry((team, id))
            .or_insert_with(|| Self::create_initial_filter(process_noise_p, process_noise_v, measurement_noise));

        // Calcular aceleración preliminar
        let (prev_x, prev_y) = (filter.x[0], filter.x[1]);
        let (prev_vx, prev_vy) = (filter.x[4], filter.x[5]);
        let v_inst_x = (x - prev_x) / dt;
        let v_inst_y = (y - prev_y) / dt;
        let a_inst_x = (v_inst_x - prev_vx) / dt;
        let a_inst_y = (v_inst_y - prev_vy) / dt;
        let a_magnitude = (a_inst_x*a_inst_x + a_inst_y*a_inst_y).sqrt();

        // Si la aceleración es enorme, es un kicker o choque
        if a_magnitude > 20.0 {
            filter.trigger_transient(5);
        }

        // Adaptar matrices y proceder con predict/update
        let current_v_mag = (prev_vx*prev_vx + prev_vy*prev_vy).sqrt();
        let current_confidence = 1.0; // TODO: Enlazar con telemetría de confianza de SSL-Vision
        filter.adapt_matrices(current_v_mag, current_confidence);

        let (_, _, _, vx, vy, omega, innovation, predict_time, update_time) = filter.filter_pose(x, y, theta, dt);
        
        // ... (código existente para registrar telemetría y retornar pose) ...
        (x, y, theta, vx, vy, omega)
    }
    ```

---

## 4. Plan de Pruebas y Validación

Para garantizar el correcto funcionamiento del AKF y evitar regresiones:

1.  **Pruebas Unitarias (`cargo test`):**
    *   Verificar que `trigger_transient` incrementa las componentes de velocidad de la matriz $Q_k$.
    *   Verificar que tras 5 llamadas a `predict`/`update`, las matrices regresan exactamente a $Q_0$.
2.  **Pruebas en el Simulador (grSim):**
    *   Generar un disparo simulado con el kicker a máxima potencia.
    *   Graficar $X_{\text{estimada}}$ vs Tiempo. La curva no debe presentar el pico de 4 metros de sobredisparo, acoplándose al valor real en máximo 2 fotogramas.
    *   Verificar que al detenerse la pelota a baja velocidad, los robots no presenten oscilaciones (ruido de velocidad residual menor a $0.02\text{ m/s}$).

---

## 5. Ruta de Desarrollo Incremental (Integración de Literatura)

Para evitar introducir demasiada complejidad matemática de golpe, el desarrollo se estructurará de forma incremental en tres fases de madurez del tracker, combinando las propuestas complementarias de *[Lai (2024)](file:///home/mrepetto/Documentos/sysmic/SYSMIC%20DOCS/08.%20Paper%20Notes/2024%20Lai%20-%20Adaptive%20Kalman%20Filtering.md)* y *[Almuhaihi (2026)](file:///home/mrepetto/Documentos/sysmic/SYSMIC%20DOCS/08.%20Paper%20Notes/2026%20Almuhaihi%20-%20Adaptive%20Kalman%20Filter%20Coloring%20Dynamics.md)*:

### Fase 1: Filtro Adaptativo Base (Heurístico)
*   **Objetivo:** Implementar la base descrita en esta especificación para mitigar el lag inicial de 8-10 frames en disparos y oclusiones.
*   **Mecánica:**
    *   Mantener el EKF de 7 estados en `tracker.rs`.
    *   Detectar transitorios rápidos mediante un umbral simple de aceleración numérica discreta ($a_{\text{inst}} > 20\text{ m/s}^2$).
    *   Durante el transitorio ($N_{\text{decay}} = 5$), escalar $Q$ mediante un multiplicador estático ($\alpha = 1000$).
    *   Escalar $R$ según la velocidad lineal y confianza reportada.
*   **Estado:** Base inicial de validación rápida.

### Fase 2: Robustez ante Colisiones e Impactos (Lai - RVFF-KF)
*   **Objetivo:** Sustituir la detección heurística y los multiplicadores rígidos de $Q$ por una adaptación estadística rigurosa ante colisiones (balón-robot, balón-pared o robots entre sí).
*   **Mecánica:**
    *   **Distancia de Mahalanobis:** En lugar de derivar la posición numéricamente (lo cual es muy sensible al ruido), evaluar el residuo de innovación posterior $\nu_k = z_k - H_k \hat{x}_{k|k-1}$ de forma estadística:
        $$\epsilon_k^2 = \nu_k^T (H_k P_k^- H_k^T + R_k)^{-1} \nu_k$$
        Si $\epsilon_k^2$ supera el umbral chi-cuadrado para 3 grados de libertad, se detecta una perturbación impulsiva (colisión).
    *   **Factor de Olvido Variable Robusto (RVFF):** Calcular recursivamente un $\lambda_k \in (0.9, 1.0)$ en función de la excitación de los residuos.
    *   **Propagación de Covarianza:** Ajustar la matriz de covarianza de estado a priori:
        $$P_{\text{forget}, k} = P_k + \left(\frac{1}{\lambda_k} - 1\right)P_k$$
        $$P_{k+1|k} = F_k P_{\text{forget}, k} F_k^T + Q_0$$
        Esto incrementa la covarianza de forma proporcional a la magnitud real del impacto, logrando convergencia instantánea ante cualquier tipo de rebote.

### Fase 3: Mitigación de Jitter en Estado Estacionario (Almuhaihi - IWAKF)
*   **Objetivo:** Eliminar el ruido de alta frecuencia (jitter) y oscilaciones de velocidad a baja velocidad, previniendo el congelamiento de robots sin perder reactividad.
*   **Mecánica:**
    *   **Aumento de Estados:** Extender el vector de estados a 11 variables para modelar la dinámica de coloración de ruido (filtro AR de segundo orden en aceleraciones):
        $$x_{\text{augmented}} = [x, y, \sin\theta, \cos\theta, vx, vy, \omega, c_{vx}, c_{vy}, c_{\omega}, c_{\theta}]^T$$
    *   **Blanqueamiento de Innovaciones:** Implementar el optimizador online del parámetro de coloración $\gamma$ minimizando la autocorrelación empírica $J(\gamma)$ de las últimas innovaciones.
    *   **Cómputo Asíncrono:** Correr la optimización de $\gamma$ a menor frecuencia (ej. 10 Hz) en un hilo paralelo para cumplir con las restricciones de 16.6 ms de ciclo de control.
