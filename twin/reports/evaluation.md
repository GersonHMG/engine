# Digital twin evaluation (synthetic robots)

Errors are against the noise-free truth on held-out test sessions; models only saw noisy vision.

## bad_wheel: FAIL

624 open-loop rollouts of 2 s from the test sessions.

| model | pos 0.5 s | pos 1 s | pos 1 s p90 | pos 2 s | heading 1 s | one-step vel RMSE (vt, vn, ω) |
|---|---|---|---|---|---|---|
| ssl-sim (default) | 22.6 cm | 43.6 cm | 106.7 cm | 74.7 cm | 25.24° | [0.0452, 0.0535, 0.4743] |
| ssl-sim (fitted) | 5.7 cm | 17.4 cm | 54.5 cm | 56.2 cm | 29.06° | [0.0181, 0.019, 0.1965] |
| ARX (linear) | 7.6 cm | 22.6 cm | 42.3 cm | 54.7 cm | 35.86° | [0.0197, 0.0168, 0.1804] |
| GRU twin | 0.9 cm | 1.8 cm | 4.8 cm | 4.2 cm | 1.19° | [0.0127, 0.0121, 0.114] |

GRU twin, 1-s open-loop error by the fastest command in the first second:

| regime | windows | pos 1 s median | heading 1 s median |
|---|---|---|---|
| ≤ 1 m/s | 137 | 0.4 cm | 0.36° |
| 1–2 m/s | 233 | 2.2 cm | 1.68° |
| 2–2.5 m/s | 126 | 2.0 cm | 1.17° |
| > 2.5 m/s, spin ≤ 3 rad/s | 61 | 2.1 cm | 1.90° |
| > 2.5 m/s, spin > 3 rad/s | 67 | 3.4 cm | 3.73° |

Noise floor: raw vision vs truth 3.8 mm; smoothed start pose vs truth 1.6 mm.

Residual structure (R² of one-step residuals explained by state + commands, vt, vn, ω): [0.012, 0.003, 0.02]

| maneuver (1.5 s + 0.5 s stop) | real end (x, y, θ°) | twin end (x, y, θ°) | end error | heading error |
|---|---|---|---|---|
| forward 1 m/s | [1.423, 0.327, 4.3] | [1.422, 0.3, 3.9] | 2.7 cm | 0.40° |
| forward 2.5 m/s | [1.363, 2.213, 55.8] | [1.308, 2.218, 55.9] | 5.5 cm | 0.09° |
| backward 1.5 m/s | [-1.744, 1.038, -20.4] | [-1.727, 1.052, -20.1] | 2.3 cm | 0.23° |
| left 1 m/s | [0.418, 1.306, -10.8] | [0.446, 1.295, -11.8] | 3.0 cm | 1.02° |
| right 2 m/s | [1.105, -1.284, 75.2] | [1.096, -1.291, 73.8] | 1.1 cm | 1.39° |
| left creep 0.15 m/s | [0.011, 0.183, -1.7] | [0.009, 0.195, -1.0] | 1.2 cm | 0.73° |
| forward creep 0.1 m/s | [0.13, 0.007, 0.8] | [0.145, 0.003, 0.2] | 1.6 cm | 0.55° |
| diagonal 1.5 m/s | [1.739, 1.014, -9.5] | [1.733, 0.998, -9.3] | 1.6 cm | 0.14° |
| spin 4 rad/s | [0.001, 0.001, 343.4] | [0.001, 0.004, 342.7] | 0.3 cm | 0.69° |
| forward 1 m/s + spin 3 rad/s | [-0.193, 0.709, 222.0] | [-0.249, 0.682, 224.3] | 6.2 cm | 2.29° |

Free runs: whole test sessions, the twin fed only the logged commands and its own predictions.

| session | length | vel RMSE vt, vn, ω | error growth (last/first 30 s) | final drift |
|---|---|---|---|---|
| move_to | 120.0 s | [0.0367, 0.0409, 0.14] | 2.47× | 235 cm |
| prbs | 120.0 s | [0.0645, 0.0663, 0.3372] | 1.29× | 399 cm |
| spin_translate | 120.0 s | [0.0428, 0.0457, 0.1796] | 0.84× | 86 cm |
| steps | 120.0 s | [0.0207, 0.019, 0.1582] | 0.63× | 107 cm |

Closed loop (6 s per leg): go_to controller fed back with the twin's own pose vs. the same controller on the reference robot (through vision).

| leg | real settles (5 cm) | twin settles | max path separation |
|---|---|---|---|
| (-2.0, 0.0) -> (1.0, 0.0) @ 2.0 m/s | 2.83 s | 2.83 s | 1.3 cm |
| (0.0, -1.5) -> (0.0, 1.5) @ 2.0 m/s | 3.22 s | 3.27 s | 1.7 cm |
| (-1.0, -1.0) -> (1.0, 1.0) @ 2.5 m/s | 3.28 s | 3.32 s | 2.9 cm |
| (1.5, 0.5) -> (-1.5, -0.5) @ 3.0 m/s | 3.33 s | 3.18 s | 12.2 cm |
| (0.0, 0.0) -> (0.3, 0.2) @ 1.0 m/s | 0.82 s | 0.90 s | 1.8 cm |
| (-2.0, 1.0) -> (2.0, -1.0) @ 3.0 m/s | 3.88 s | 3.97 s | 12.7 cm |

| check | result |
|---|---|
| 1 s median position error ≤ 3 cm | pass |
| 1 s median heading error ≤ 2.0° | pass |
| ≥ 50 % better than fitted ssl-sim at 1 s | pass |
| every maneuver ends within 5 cm and 3.0° | FAIL |
| 2-min free runs: vel RMSE ≤ 0.1 m/s, ω ≤ 0.5 rad/s, error growth ≤ 1.5× | FAIL |
| closed loop: settles within 0.15 s of the real robot, paths within 5 cm | FAIL |

## lateral: FAIL

691 open-loop rollouts of 2 s from the test sessions.

| model | pos 0.5 s | pos 1 s | pos 1 s p90 | pos 2 s | heading 1 s | one-step vel RMSE (vt, vn, ω) |
|---|---|---|---|---|---|---|
| ssl-sim (default) | 23.0 cm | 38.4 cm | 119.7 cm | 73.1 cm | 17.97° | [0.0421, 0.0602, 0.397] |
| ssl-sim (fitted) | 8.2 cm | 26.2 cm | 61.2 cm | 65.2 cm | 17.44° | [0.0202, 0.0205, 0.1574] |
| ARX (linear) | 7.1 cm | 19.4 cm | 34.5 cm | 34.3 cm | 10.51° | [0.0204, 0.0194, 0.1371] |
| GRU twin | 0.7 cm | 1.3 cm | 4.1 cm | 2.8 cm | 1.13° | [0.0129, 0.0121, 0.1067] |

GRU twin, 1-s open-loop error by the fastest command in the first second:

| regime | windows | pos 1 s median | heading 1 s median |
|---|---|---|---|
| ≤ 1 m/s | 174 | 0.7 cm | 0.41° |
| 1–2 m/s | 263 | 1.5 cm | 1.53° |
| 2–2.5 m/s | 96 | 1.3 cm | 1.89° |
| > 2.5 m/s, spin ≤ 3 rad/s | 72 | 1.4 cm | 1.33° |
| > 2.5 m/s, spin > 3 rad/s | 86 | 2.7 cm | 4.03° |

Noise floor: raw vision vs truth 3.8 mm; smoothed start pose vs truth 1.6 mm.

Residual structure (R² of one-step residuals explained by state + commands, vt, vn, ω): [0.005, 0.009, 0.084]

| maneuver (1.5 s + 0.5 s stop) | real end (x, y, θ°) | twin end (x, y, θ°) | end error | heading error |
|---|---|---|---|---|
| forward 1 m/s | [1.444, -0.0, -0.0] | [1.442, -0.01, -0.6] | 1.1 cm | 0.63° |
| forward 2.5 m/s | [2.955, 0.0, 0.0] | [2.926, -0.027, -1.2] | 4.0 cm | 1.15° |
| backward 1.5 m/s | [-2.119, -0.0, 0.0] | [-2.124, -0.004, 0.4] | 0.7 cm | 0.41° |
| left 1 m/s | [-0.026, 1.145, -0.2] | [-0.022, 1.13, 1.0] | 1.5 cm | 1.26° |
| right 2 m/s | [-0.104, -1.222, -31.3] | [-0.123, -1.219, -31.1] | 1.9 cm | 0.21° |
| left creep 0.15 m/s | [0.002, 0.141, -1.5] | [-0.002, 0.146, -0.9] | 0.7 cm | 0.57° |
| forward creep 0.1 m/s | [0.132, -0.0, -0.0] | [0.143, -0.003, -0.0] | 1.1 cm | 0.01° |
| diagonal 1.5 m/s | [1.208, 0.401, -24.1] | [1.193, 0.369, -24.9] | 3.5 cm | 0.82° |
| spin 4 rad/s | [0.0, 0.0, 343.5] | [0.001, 0.002, 343.6] | 0.2 cm | 0.16° |
| forward 1 m/s + spin 3 rad/s | [-0.237, 0.515, 237.2] | [-0.211, 0.527, 237.0] | 2.9 cm | 0.18° |

Free runs: whole test sessions, the twin fed only the logged commands and its own predictions.

| session | length | vel RMSE vt, vn, ω | error growth (last/first 30 s) | final drift |
|---|---|---|---|---|
| move_to | 120.0 s | [0.0607, 0.0263, 0.1596] | 1.38× | 253 cm |
| prbs | 120.0 s | [0.0482, 0.0423, 0.3071] | 1.14× | 339 cm |
| spin_translate | 120.0 s | [0.0339, 0.0376, 0.1535] | 2.08× | 249 cm |
| steps | 120.0 s | [0.0197, 0.0127, 0.1068] | 1.0× | 70 cm |

Closed loop (6 s per leg): go_to controller fed back with the twin's own pose vs. the same controller on the reference robot (through vision).

| leg | real settles (5 cm) | twin settles | max path separation |
|---|---|---|---|
| (-2.0, 0.0) -> (1.0, 0.0) @ 2.0 m/s | 2.23 s | 2.15 s | 2.4 cm |
| (0.0, -1.5) -> (0.0, 1.5) @ 2.0 m/s | 3.80 s | 3.82 s | 2.3 cm |
| (-1.0, -1.0) -> (1.0, 1.0) @ 2.5 m/s | 3.30 s | 3.32 s | 1.6 cm |
| (1.5, 0.5) -> (-1.5, -0.5) @ 3.0 m/s | 2.70 s | 2.58 s | 5.2 cm |
| (0.0, 0.0) -> (0.3, 0.2) @ 1.0 m/s | 0.92 s | 0.98 s | 1.2 cm |
| (-2.0, 1.0) -> (2.0, -1.0) @ 3.0 m/s | 5.10 s | 5.13 s | 6.3 cm |

| check | result |
|---|---|
| 1 s median position error ≤ 3 cm | pass |
| 1 s median heading error ≤ 2.0° | pass |
| ≥ 50 % better than fitted ssl-sim at 1 s | pass |
| every maneuver ends within 5 cm and 3.0° | pass |
| 2-min free runs: vel RMSE ≤ 0.1 m/s, ω ≤ 0.5 rad/s, error growth ≤ 1.5× | FAIL |
| closed loop: settles within 0.15 s of the real robot, paths within 5 cm | FAIL |

## perfect: FAIL

690 open-loop rollouts of 2 s from the test sessions.

| model | pos 0.5 s | pos 1 s | pos 1 s p90 | pos 2 s | heading 1 s | one-step vel RMSE (vt, vn, ω) |
|---|---|---|---|---|---|---|
| ssl-sim (default) | 20.1 cm | 37.2 cm | 93.0 cm | 65.3 cm | 13.49° | [0.0402, 0.0495, 0.39] |
| ssl-sim (fitted) | 4.4 cm | 14.2 cm | 51.1 cm | 41.6 cm | 12.95° | [0.0201, 0.0198, 0.151] |
| ARX (linear) | 7.3 cm | 22.0 cm | 46.0 cm | 44.1 cm | 12.08° | [0.0239, 0.0195, 0.1476] |
| GRU twin | 0.8 cm | 1.6 cm | 5.3 cm | 3.3 cm | 1.04° | [0.0144, 0.0136, 0.1091] |

GRU twin, 1-s open-loop error by the fastest command in the first second:

| regime | windows | pos 1 s median | heading 1 s median |
|---|---|---|---|
| ≤ 1 m/s | 150 | 0.5 cm | 0.42° |
| 1–2 m/s | 277 | 1.8 cm | 1.04° |
| 2–2.5 m/s | 122 | 1.8 cm | 1.05° |
| > 2.5 m/s, spin ≤ 3 rad/s | 69 | 1.6 cm | 1.24° |
| > 2.5 m/s, spin > 3 rad/s | 72 | 3.4 cm | 3.11° |

Noise floor: raw vision vs truth 4.3 mm; smoothed start pose vs truth 4.0 mm.

Residual structure (R² of one-step residuals explained by state + commands, vt, vn, ω): [0.039, 0.022, 0.058]

| maneuver (1.5 s + 0.5 s stop) | real end (x, y, θ°) | twin end (x, y, θ°) | end error | heading error |
|---|---|---|---|---|
| forward 1 m/s | [1.477, -0.0, -0.0] | [1.473, -0.015, -0.3] | 1.6 cm | 0.28° |
| forward 2.5 m/s | [3.372, -0.0, 0.0] | [3.356, -0.072, -0.6] | 7.4 cm | 0.59° |
| backward 1.5 m/s | [-2.196, -0.0, 0.0] | [-2.204, 0.03, -0.4] | 3.1 cm | 0.43° |
| left 1 m/s | [-0.005, 1.409, -1.6] | [0.017, 1.412, -1.7] | 2.2 cm | 0.04° |
| right 2 m/s | [-0.316, -2.536, 2.1] | [-0.373, -2.526, 0.2] | 5.8 cm | 1.84° |
| left creep 0.15 m/s | [0.001, 0.193, -0.6] | [0.002, 0.198, -0.6] | 0.5 cm | 0.06° |
| forward creep 0.1 m/s | [0.135, -0.0, -0.0] | [0.146, -0.001, 0.0] | 1.1 cm | 0.04° |
| diagonal 1.5 m/s | [1.718, 1.046, -6.4] | [1.691, 1.076, -5.8] | 4.0 cm | 0.61° |
| spin 4 rad/s | [0.001, 0.0, 343.5] | [0.001, 0.0, 343.4] | 0.1 cm | 0.13° |
| forward 1 m/s + spin 3 rad/s | [-0.126, 0.614, 228.8] | [-0.141, 0.557, 234.2] | 5.9 cm | 5.47° |

Free runs: whole test sessions, the twin fed only the logged commands and its own predictions.

| session | length | vel RMSE vt, vn, ω | error growth (last/first 30 s) | final drift |
|---|---|---|---|---|
| move_to | 120.0 s | [0.0282, 0.0282, 0.1288] | 1.13× | 158 cm |
| prbs | 120.0 s | [0.0939, 0.0939, 0.325] | 2.42× | 443 cm |
| spin_translate | 120.0 s | [0.0359, 0.0431, 0.1933] | 0.86× | 182 cm |
| steps | 120.0 s | [0.0156, 0.0161, 0.1186] | 1.93× | 24 cm |

Closed loop (6 s per leg): go_to controller fed back with the twin's own pose vs. the same controller on the reference robot (through vision).

| leg | real settles (5 cm) | twin settles | max path separation |
|---|---|---|---|
| (-2.0, 0.0) -> (1.0, 0.0) @ 2.0 m/s | 2.07 s | 2.03 s | 1.4 cm |
| (0.0, -1.5) -> (0.0, 1.5) @ 2.0 m/s | 2.20 s | 2.20 s | 0.9 cm |
| (-1.0, -1.0) -> (1.0, 1.0) @ 2.5 m/s | 2.85 s | 2.88 s | 2.1 cm |
| (1.5, 0.5) -> (-1.5, -0.5) @ 3.0 m/s | 2.90 s | 2.70 s | 10.6 cm |
| (0.0, 0.0) -> (0.3, 0.2) @ 1.0 m/s | 0.80 s | 0.88 s | 1.6 cm |
| (-2.0, 1.0) -> (2.0, -1.0) @ 3.0 m/s | 3.73 s | 3.80 s | 8.6 cm |

| check | result |
|---|---|
| 1 s median position error ≤ 3 cm | pass |
| 1 s median heading error ≤ 2.0° | pass |
| ≥ 50 % better than fitted ssl-sim at 1 s | pass |
| every maneuver ends within 5 cm and 3.0° | FAIL |
| 2-min free runs: vel RMSE ≤ 0.1 m/s, ω ≤ 0.5 rad/s, error growth ≤ 1.5× | FAIL |
| closed loop: settles within 0.15 s of the real robot, paths within 5 cm | FAIL |
