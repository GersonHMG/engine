# Context-adaptive twin on held-out robots

Checkpoint step 3250. None of these robots was in training. Errors are against the noise-free truth on two test sessions separate from the calibration drive.

| robot | 0 s ctx | 1 s ctx | 5 s ctx | 30 s ctx | 120 s ctx | 30 s + TTA | oracle z (true hardware) | fitted physics (120 s) | pos 2 s (30 s ctx) | heading 1 s (30 s ctx) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| bad_wheel | 11.1 | 8.8 | 8.8 | 3.9 | 3.5 | 1.8 | 2.5 | 13.9 | 8.7 | 3.22° |
| bad_wheel_lateral | 26.0 | 6.4 | 6.5 | 2.9 | 2.9 | 2.9 | 2.4 | 22.5 | 5.6 | 2.58° |
| degrading_wheel | 11.7 | 16.7 | 7.5 | 8.4 | 7.0 | 7.6 | 6.3 | 17.3 | 20.5 | 5.86° |
| heavy_bad_wheel | 1.4 | 1.5 | 1.5 | 1.0 | 0.9 | 0.7 | 0.7 | 2.0 | 2.2 | 1.14° |
| lateral | 4.6 | 3.2 | 3.1 | 2.3 | 1.8 | 2.6 | 1.6 | 15.8 | 5.7 | 2.56° |
| ood_dead_wheel | 2.7 | 1.6 | 1.9 | 1.4 | 1.0 | 1.3 | 1.2 | 3.8 | 2.7 | 1.79° |
| ood_slow_radio | 10.8 | 6.3 | 6.8 | 6.4 | 6.3 | 3.3 | 4.5 | 6.2 | 13.4 | 5.25° |
| perfect | 9.7 | 20.3 | 21.4 | 4.7 | 4.4 | 3.1 | 3.4 | 13.0 | 9.7 | 1.80° |
| two_bad_wheels | 32.3 | 17.8 | 16.3 | 8.9 | 6.6 | 4.4 | 4.5 | 28.9 | 21.3 | 3.07° |

Position errors in cm (median, 1 s open-loop unless noted). TTA = test-time adaptation of z on the first 30 s of the calibration drive.

Degrading wheel, second half of the test sessions: rolling 30 s context 4.6 cm vs. healthy calibration 13.2 cm.

| robot | z std (30 s ctx) | worst maneuver (2 s) | closed loop worst separation | closed loop worst settle diff |
|---|---|---|---|---|
| bad_wheel | 0.445 | backward 1.5 m/s: 33.1 cm, 6.1° | (-2.0, 1.0) -> (2.0, -1.0) @ 3.0: 77.6 cm | 0.47 s |
| bad_wheel_lateral | 0.492 | forward 2.5 m/s: 35.7 cm, 1.1° | (-2.0, 1.0) -> (2.0, -1.0) @ 3.0: 60.6 cm | 0.52 s |
| degrading_wheel | 0.383 | right 2 m/s: 44.6 cm, 18.0° | (-2.0, 1.0) -> (2.0, -1.0) @ 3.0: 30.8 cm | 0.72 s |
| heavy_bad_wheel | 0.408 | backward 1.5 m/s: 31.0 cm, 2.8° | (-2.0, 1.0) -> (2.0, -1.0) @ 3.0: 59.0 cm | 0.62 s |
| lateral | 0.590 | forward 2.5 m/s: 89.1 cm, 21.8° | (0.0, -1.5) -> (0.0, 1.5) @ 2.0: 70.5 cm | 0.80 s |
| ood_dead_wheel | 0.457 | forward 2.5 m/s: 133.2 cm, 162.4° | (-2.0, 1.0) -> (2.0, -1.0) @ 3.0: 108.2 cm | 0.25 s |
| ood_slow_radio | 0.431 | right 2 m/s: 110.0 cm, 35.4° | (1.5, 0.5) -> (-1.5, -0.5) @ 3.0: 82.8 cm | 1.52 s |
| perfect | 0.410 | forward 2.5 m/s: 63.7 cm, 9.9° | (-2.0, 1.0) -> (2.0, -1.0) @ 3.0: 57.6 cm | 0.48 s |
| two_bad_wheels | 0.492 | right 2 m/s: 44.9 cm, 30.8° | (-2.0, 1.0) -> (2.0, -1.0) @ 3.0: 83.0 cm | 0.83 s |

| check | result |
|---|---|
| in-range robots, 30 s context: 1 s error ≤ 3 cm | FAIL |
| faulty robots: 30 s context ≤ 0.6 × no context | FAIL |
| in-range robots: ≥ 50 % better than fitted physics | pass |
| degrading wheel: rolling context beats the stale calibration | pass |
