# Context-adaptive twin on held-out robots

Checkpoint step 3250. None of these robots was in training. Errors are against the noise-free truth on two test sessions separate from the calibration drive.

| robot | 0 s ctx | 1 s ctx | 5 s ctx | 30 s ctx | 120 s ctx | 30 s + TTA | oracle z (true hardware) | fitted physics (120 s) | pos 2 s (30 s ctx) | heading 1 s (30 s ctx) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| bad_wheel | 11.9 | 8.7 | 8.5 | 3.9 | 3.9 | 2.0 | 2.5 | 13.9 | 8.9 | 2.39° |
| bad_wheel_lateral | 25.6 | 7.8 | 6.7 | 3.1 | 2.7 | 2.8 | 2.4 | 22.5 | 6.3 | 2.34° |
| degrading_wheel | 10.9 | 16.2 | 7.2 | 8.1 | 7.5 | 6.4 | 6.7 | 17.3 | 21.6 | 6.75° |
| heavy_bad_wheel | 1.4 | 1.3 | 1.4 | 0.9 | 0.8 | 0.7 | 0.8 | 2.0 | 2.1 | 1.28° |
| lateral | 4.4 | 3.2 | 3.2 | 2.7 | 1.8 | 2.7 | 1.6 | 15.8 | 5.8 | 2.37° |
| ood_dead_wheel | 2.8 | 1.8 | 1.9 | 1.5 | 1.1 | 1.1 | 1.0 | 3.8 | 2.9 | 1.79° |
| ood_slow_radio | 11.1 | 5.8 | 6.0 | 6.5 | 6.3 | 3.7 | 4.7 | 6.2 | 14.6 | 5.59° |
| perfect | 9.6 | 20.3 | 20.1 | 4.2 | 4.3 | 2.9 | 3.5 | 13.0 | 9.9 | 1.92° |
| two_bad_wheels | 31.9 | 16.7 | 15.0 | 8.4 | 6.1 | 3.2 | 4.3 | 28.9 | 20.4 | 3.94° |

Position errors in cm (median, 1 s open-loop unless noted). TTA = test-time adaptation of z on the first 30 s of the calibration drive.

Degrading wheel, second half of the test sessions: rolling 30 s context 4.8 cm vs. healthy calibration 13.4 cm.

From rest: 10 maneuvers (end error after 2 s) and 6 closed-loop legs (max path separation), median / worst, in cm. 'estimated' = z from 30 s of calibration, 'true' = z from the true hardware.

| robot | maneuvers, estimated z | maneuvers, true z | closed loop, estimated z | closed loop, true z | worst maneuver (estimated z) |
|---|---|---|---|---|---|
| bad_wheel | 9.5 / 22.0 | 6.6 / 46.3 | 30.1 / 55.7 | 8.9 / 18.7 | backward 1.5 m/s: 22.0 cm, 3.6° |
| bad_wheel_lateral | 11.8 / 45.9 | 7.0 / 52.8 | 33.5 / 63.8 | 14.5 / 59.7 | forward 2.5 m/s: 45.9 cm, 27.6° |
| degrading_wheel | 7.1 / 86.4 | 3.9 / 29.9 | 11.2 / 23.6 | 4.7 / 13.5 | right 2 m/s: 86.4 cm, 25.4° |
| heavy_bad_wheel | 8.2 / 26.3 | 6.0 / 25.1 | 18.2 / 61.2 | 8.2 / 24.7 | forward 1 m/s + spin 3 rad/s: 26.3 cm, 1.8° |
| lateral | 13.8 / 94.1 | 6.0 / 52.5 | 53.8 / 86.5 | 19.0 / 40.8 | forward 2.5 m/s: 94.1 cm, 16.3° |
| ood_dead_wheel | 23.9 / 128.7 | 9.5 / 39.2 | 50.8 / 105.4 | 12.5 / 33.9 | forward 2.5 m/s: 128.7 cm, 149.6° |
| ood_slow_radio | 21.8 / 197.6 | 6.5 / 26.5 | 40.1 / 62.7 | 26.1 / 92.2 | right 2 m/s: 197.6 cm, 74.0° |
| perfect | 7.2 / 76.8 | 3.9 / 29.9 | 7.6 / 48.9 | 4.7 / 13.5 | forward 2.5 m/s: 76.8 cm, 6.9° |
| two_bad_wheels | 17.6 / 46.4 | 4.9 / 20.3 | 30.6 / 97.3 | 13.7 / 37.8 | forward 2.5 m/s: 46.4 cm, 15.7° |

| check | result |
|---|---|
| in-range robots, 30 s context: 1 s error ≤ 3 cm | FAIL |
| faulty robots: 30 s context ≤ 0.6 × no context | FAIL |
| in-range robots: ≥ 50 % better than fitted physics | pass |
| degrading wheel: rolling context beats the stale calibration | pass |
