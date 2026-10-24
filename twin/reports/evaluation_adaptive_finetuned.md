# Context-adaptive twin on held-out robots

Checkpoint step 1500. None of these robots was in training. Errors are against the noise-free truth on two test sessions separate from the calibration drive.

| robot | 0 s ctx | 1 s ctx | 5 s ctx | 30 s ctx | 120 s ctx | 30 s + TTA | oracle z (true hardware) | fitted physics (120 s) | pos 2 s (30 s ctx) | heading 1 s (30 s ctx) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| bad_wheel | 7.7 | 7.8 | 7.9 | 3.4 | 3.5 | 3.0 | 4.3 | 13.9 | 8.8 | 2.74° |
| bad_wheel_lateral | 11.9 | 7.0 | 7.0 | 3.5 | 3.4 | 2.6 | 3.7 | 22.5 | 7.2 | 2.75° |
| degrading_wheel | 10.7 | 13.6 | 7.5 | 8.2 | 7.7 | 6.4 | 7.0 | 17.3 | 20.6 | 5.87° |
| heavy_bad_wheel | 1.0 | 1.7 | 1.7 | 1.1 | 1.0 | 0.8 | 1.0 | 2.0 | 3.0 | 0.94° |
| lateral | 2.7 | 3.4 | 3.3 | 3.0 | 2.0 | 1.9 | 1.9 | 15.8 | 7.0 | 2.09° |
| ood_dead_wheel | 2.2 | 1.5 | 1.6 | 1.3 | 1.3 | 1.5 | 1.7 | 3.8 | 2.9 | 1.23° |
| ood_slow_radio | 11.0 | 6.9 | 6.9 | 7.2 | 7.0 | 4.3 | 7.5 | 6.2 | 12.8 | 4.84° |
| perfect | 14.2 | 18.6 | 17.9 | 5.0 | 4.8 | 4.0 | 9.7 | 13.0 | 10.8 | 2.40° |
| two_bad_wheels | 19.3 | 14.8 | 13.7 | 8.4 | 6.9 | 5.6 | 9.9 | 28.9 | 19.1 | 3.55° |

Position errors in cm (median, 1 s open-loop unless noted). TTA = test-time adaptation of z on the first 30 s of the calibration drive.

Degrading wheel, second half of the test sessions: rolling 30 s context 5.5 cm vs. healthy calibration 13.6 cm.

From rest: 10 maneuvers (end error after 2 s) and 6 closed-loop legs (max path separation), median / worst, in cm. 'estimated' = z from 30 s of calibration, 'true' = z from the true hardware.

| robot | maneuvers, estimated z | maneuvers, true z | closed loop, estimated z | closed loop, true z | worst maneuver (estimated z) |
|---|---|---|---|---|---|
| bad_wheel | 13.6 / 108.2 | 18.1 / 75.3 | 28.9 / 58.8 | 49.0 / 113.0 | forward 2.5 m/s: 108.2 cm, 28.1° |
| bad_wheel_lateral | 17.4 / 59.9 | 14.8 / 51.5 | 30.2 / 51.4 | 35.7 / 86.4 | forward 2.5 m/s: 59.9 cm, 8.4° |
| degrading_wheel | 8.1 / 62.4 | 14.6 / 97.8 | 11.2 / 24.5 | 59.6 / 129.9 | right 2 m/s: 62.4 cm, 18.7° |
| heavy_bad_wheel | 12.9 / 54.9 | 14.9 / 54.9 | 22.8 / 83.0 | 51.3 / 149.3 | forward 2.5 m/s: 54.9 cm, 59.4° |
| lateral | 18.8 / 92.1 | 14.0 / 84.9 | 51.0 / 57.8 | 34.9 / 96.3 | forward 2.5 m/s: 92.1 cm, 28.3° |
| ood_dead_wheel | 20.1 / 133.2 | 21.9 / 57.2 | 38.2 / 74.2 | 41.8 / 100.6 | forward 2.5 m/s: 133.2 cm, 139.2° |
| ood_slow_radio | 10.8 / 119.9 | 13.2 / 86.0 | 40.4 / 80.3 | 47.0 / 129.2 | right 2 m/s: 119.9 cm, 43.2° |
| perfect | 6.1 / 52.8 | 14.6 / 97.8 | 12.7 / 43.8 | 59.6 / 129.9 | forward 2.5 m/s: 52.8 cm, 3.9° |
| two_bad_wheels | 13.9 / 82.5 | 17.9 / 96.2 | 21.5 / 96.5 | 37.4 / 113.2 | forward 2.5 m/s: 82.5 cm, 25.5° |

| check | result |
|---|---|
| in-range robots, 30 s context: 1 s error ≤ 3 cm | FAIL |
| faulty robots: 30 s context ≤ 0.6 × no context | FAIL |
| in-range robots: ≥ 50 % better than fitted physics | FAIL |
| degrading wheel: rolling context beats the stale calibration | pass |
