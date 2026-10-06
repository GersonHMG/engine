use ssl_sim::{Ball, EventKind, Robot, RobotCommand, RobotId, SimConfig, SimError, SimState, Simulator, Team, Vec2};
use std::f64::consts::FRAC_PI_2;

const TICK: f64 = 1.0 / 60.0;
const BLUE0: RobotId = RobotId::new(Team::Blue, 0);
const YELLOW0: RobotId = RobotId::new(Team::Yellow, 0);

fn sim_with(robot: Robot, ball: Ball) -> Simulator {
    Simulator::new(SimConfig::default(), SimState::new(vec![robot], ball))
}

/// Ball parked far from the robot, so it does not interfere.
fn ball_out_of_the_way() -> Ball {
    Ball::new(Vec2::new(0.0, 2.5), Vec2::ZERO)
}

fn run(sim: &mut Simulator, seconds: f64) {
    for _ in 0..(seconds / TICK).round() as usize {
        sim.step(TICK);
    }
}

/// Steps one substep at a time until an event matching `wanted` fires, so
/// the state can be checked right at that moment.
fn run_until(sim: &mut Simulator, seconds: f64, wanted: impl Fn(&EventKind) -> bool) -> EventKind {
    let substep = sim.config().max_substep;
    for _ in 0..(seconds / substep).round() as usize {
        sim.step(substep);
        if let Some(event) = sim.take_events().into_iter().find(|e| wanted(&e.kind)) {
            return event.kind;
        }
    }
    panic!("no matching event within {seconds} s");
}

fn assert_close(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected} ± {tolerance}, got {actual}"
    );
}

#[test]
fn ball_slides_then_rolls_to_the_expected_distance() {
    let config = SimConfig::default();
    let v0 = 2.0;
    let mut sim = sim_with(
        Robot::new(BLUE0, Vec2::new(-4.0, -2.5), 0.0),
        Ball::new(Vec2::new(-3.0, 0.0), Vec2::new(v0, 0.0)),
    );

    run(&mut sim, 10.0);

    // Sliding from v0 down to 5/7·v0, then rolling to a stop.
    let v1 = v0 * 5.0 / 7.0;
    let sliding = (v0 * v0 - v1 * v1) / (2.0 * config.ball.slide_decel);
    let rolling = v1 * v1 / (2.0 * config.ball.roll_decel);
    let ball = &sim.state().ball;
    assert_close(ball.position.x - (-3.0), sliding + rolling, 0.01);
    assert_eq!(ball.velocity, Vec2::ZERO);
}

#[test]
fn robot_drives_in_its_own_frame_within_the_acceleration_limit() {
    let config = SimConfig::default();
    let mut sim = sim_with(Robot::new(BLUE0, Vec2::ZERO, FRAC_PI_2), ball_out_of_the_way());
    sim.set_command(BLUE0, RobotCommand { vt: 1.5, ..Default::default() }).unwrap();

    // Forward while facing +y means moving along +y.
    let ramp = 1.5 / config.robot.max_accel;
    run(&mut sim, ramp / 2.0);
    let robot = &sim.state().robots()[0];
    assert!(robot.velocity.y < 1.5, "should still be accelerating");

    run(&mut sim, ramp);
    let robot = &sim.state().robots()[0];
    assert_close(robot.velocity.y, 1.5, 1e-9);
    assert_close(robot.velocity.x, 0.0, 1e-9);
}

#[test]
fn robot_turns_at_the_commanded_rate() {
    let mut sim = sim_with(Robot::new(BLUE0, Vec2::ZERO, 0.0), ball_out_of_the_way());
    sim.set_command(BLUE0, RobotCommand { omega: 2.0, ..Default::default() }).unwrap();

    run(&mut sim, 1.0);

    // 2 rad/s for 1 s, minus the time spent spinning up to 2 rad/s.
    let config = SimConfig::default();
    let spin_up = 2.0 / config.robot.max_alpha;
    assert_close(sim.state().robots()[0].theta, 2.0 * (1.0 - spin_up / 2.0), 0.01);
}

#[test]
fn ball_bounces_off_the_wall() {
    let config = SimConfig::default();
    let wall = config.field.wall_x();
    let mut sim = sim_with(
        Robot::new(BLUE0, Vec2::new(-4.0, -2.5), 0.0),
        Ball::new(Vec2::new(wall - 0.2, 0.0), Vec2::new(3.0, 0.0)),
    );

    let EventKind::BallHitWall { speed, .. } =
        run_until(&mut sim, 0.5, |k| matches!(k, EventKind::BallHitWall { .. }))
    else {
        unreachable!()
    };

    let ball = &sim.state().ball;
    assert!(ball.position.x <= wall - config.ball.radius);
    assert_close(-ball.velocity.x, config.ball.wall_restitution * speed, 1e-9);
}

#[test]
fn robot_stops_at_the_wall() {
    let config = SimConfig::default();
    let mut sim = sim_with(Robot::new(BLUE0, Vec2::new(3.5, 0.0), 0.0), ball_out_of_the_way());
    sim.set_command(BLUE0, RobotCommand { vt: 2.0, ..Default::default() }).unwrap();

    run(&mut sim, 2.0);

    let robot = &sim.state().robots()[0];
    assert_close(robot.position.x, config.field.wall_x() - config.robot.radius, 1e-9);
    assert_eq!(robot.velocity.x, 0.0);

    let hits = sim
        .take_events()
        .iter()
        .filter(|e| matches!(e.kind, EventKind::RobotHitWall { .. }))
        .count();
    assert_eq!(hits, 1, "one contact, one event");
}

#[test]
fn kick_sends_the_ball_along_the_heading() {
    let config = SimConfig::default();
    let at_front = config.robot.front_distance + config.ball.radius;
    let mut sim = sim_with(
        Robot::new(BLUE0, Vec2::ZERO, FRAC_PI_2),
        Ball::new(Vec2::new(0.0, at_front), Vec2::ZERO),
    );
    sim.set_command(BLUE0, RobotCommand { kick_speed: 4.0, ..Default::default() }).unwrap();

    sim.step(TICK);

    let ball = &sim.state().ball;
    assert_close(ball.velocity.x, 0.0, 1e-9);
    assert_close(ball.velocity.y, 4.0, 0.1);
    assert_eq!(sim.state().robots()[0].command.kick_speed, 0.0, "the kick is used up");
    assert!(sim
        .take_events()
        .iter()
        .any(|e| matches!(e.kind, EventKind::Kick { speed, .. } if speed == 4.0)));
}

#[test]
fn kick_waits_until_the_ball_reaches_the_kicker() {
    let mut sim = sim_with(Robot::new(BLUE0, Vec2::new(-1.0, 0.0), 0.0), Ball::new(Vec2::ZERO, Vec2::ZERO));
    sim.set_command(BLUE0, RobotCommand { vt: 1.0, kick_speed: 3.0, ..Default::default() }).unwrap();

    sim.step(TICK);
    assert_eq!(sim.state().ball.velocity, Vec2::ZERO, "too far to kick yet");

    // The kicker reaches a little past the face, so it fires just before contact.
    run_until(&mut sim, 1.5, |k| matches!(k, EventKind::Kick { .. }));

    let state = sim.state();
    let config = SimConfig::default();
    let gap = state.ball.position.x - state.robots()[0].position.x;
    assert!(gap <= config.robot.front_distance + config.ball.radius + config.robot.kick_reach);
    assert_close(state.ball.velocity.x, 3.0 + state.robots()[0].velocity.x, 0.01);
}

#[test]
fn robot_pushes_the_ball_without_overlapping_it() {
    let config = SimConfig::default();
    let mut sim = sim_with(Robot::new(BLUE0, Vec2::new(-1.0, 0.0), FRAC_PI_2), Ball::new(Vec2::ZERO, Vec2::ZERO));
    // Facing +y but driving sideways (+x): the round side hits the ball.
    sim.set_command(BLUE0, RobotCommand { vn: -1.0, ..Default::default() }).unwrap();

    for _ in 0..90 {
        sim.step(TICK);
        let state = sim.state();
        let gap = (state.ball.position - state.robots()[0].position).length();
        assert!(gap >= config.robot.radius + config.ball.radius - 1e-6, "overlap: gap = {gap}");
    }

    assert!(sim.state().ball.position.x > 0.0, "ball was pushed forward");
    let kinds: Vec<_> = sim.take_events().into_iter().map(|e| e.kind).collect();
    assert!(kinds.contains(&EventKind::RobotTouchedBall { robot: BLUE0, on_front: false }));
}

#[test]
fn snapshot_restore_replays_exactly() {
    let mut sim = Simulator::new(SimConfig::default(), SimState::default());
    sim.set_command(BLUE0, RobotCommand { vt: 1.2, vn: 0.3, omega: 0.5, kick_speed: 3.0 }).unwrap();
    run(&mut sim, 0.5);

    let snapshot = sim.state().clone();
    run(&mut sim, 2.0);
    let first = sim.state().clone();

    sim.restore(snapshot);
    run(&mut sim, 2.0);
    assert_eq!(sim.state(), &first);
}


#[test]
fn robots_do_not_drive_through_each_other() {
    let config = SimConfig::default();
    let mut sim = Simulator::new(
        SimConfig::default(),
        SimState::new(
            vec![
                Robot::new(BLUE0, Vec2::new(-1.0, 0.0), 0.0),
                Robot::new(YELLOW0, Vec2::new(1.0, 0.0), std::f64::consts::PI),
            ],
            ball_out_of_the_way(),
        ),
    );
    // Head-on: both drive forward, towards each other.
    sim.set_command(BLUE0, RobotCommand { vt: 1.5, ..Default::default() }).unwrap();
    sim.set_command(YELLOW0, RobotCommand { vt: 1.5, ..Default::default() }).unwrap();

    for _ in 0..180 {
        sim.step(TICK);
        let robots = sim.state().robots();
        let gap = (robots[1].position - robots[0].position).length();
        assert!(gap >= 2.0 * config.robot.radius - 1e-6, "overlap: gap = {gap}");
    }

    // Equal and opposite pushes: they meet in the middle.
    let robots = sim.state().robots();
    assert_close(robots[0].position.x + robots[1].position.x, 0.0, 1e-6);
}

#[test]
fn robots_are_added_and_removed_by_id() {
    let mut sim = Simulator::new(SimConfig::default(), SimState::default());

    sim.add_robot(YELLOW0, Vec2::new(1.0, 0.0), 0.0).unwrap();
    assert_eq!(sim.add_robot(YELLOW0, Vec2::ZERO, 0.0), Err(SimError::DuplicateRobot(YELLOW0)));
    let ids: Vec<_> = sim.state().robots().iter().map(|r| r.id).collect();
    assert_eq!(ids, [BLUE0, YELLOW0], "sorted by id");

    sim.remove_robot(BLUE0).unwrap();
    assert_eq!(sim.remove_robot(BLUE0), Err(SimError::UnknownRobot(BLUE0)));
    assert_eq!(
        sim.set_command(BLUE0, RobotCommand::default()),
        Err(SimError::UnknownRobot(BLUE0))
    );
    assert!(sim.state().robot(YELLOW0).is_some());
}

#[test]
fn each_robot_kicks_with_its_own_command() {
    let config = SimConfig::default();
    let at_front = config.robot.front_distance + config.ball.radius;
    let mut sim = Simulator::new(
        SimConfig::default(),
        SimState::new(
            vec![
                Robot::new(BLUE0, Vec2::new(-2.0, -2.0), 0.0),
                Robot::new(YELLOW0, Vec2::new(at_front, 0.0), std::f64::consts::PI),
            ],
            Ball::new(Vec2::ZERO, Vec2::ZERO),
        ),
    );
    sim.set_command(BLUE0, RobotCommand { kick_speed: 4.0, ..Default::default() }).unwrap();
    sim.set_command(YELLOW0, RobotCommand { kick_speed: 2.0, ..Default::default() }).unwrap();

    sim.step(TICK);

    // Only yellow 0 has the ball at its kicker; it kicks towards -x.
    assert_close(sim.state().ball.velocity.x, -2.0, 0.1);
    let kicks: Vec<_> = sim
        .take_events()
        .into_iter()
        .filter_map(|e| match e.kind {
            EventKind::Kick { robot, .. } => Some(robot),
            _ => None,
        })
        .collect();
    assert_eq!(kicks, [YELLOW0]);
}

#[test]
fn config_validation_names_the_bad_parameter() {
    assert_eq!(SimConfig::default().validate(), Ok(()));

    let config = SimConfig { max_substep: 0.0, ..Default::default() };
    assert_eq!(config.validate(), Err("max_substep must be > 0, got 0".to_string()));

    let mut config = SimConfig::default();
    config.robot.front_distance = 0.2;
    assert!(config.validate().unwrap_err().starts_with("robot.front_distance"));

    let mut config = SimConfig::default();
    config.ball.wall_restitution = f64::NAN;
    assert!(config.validate().unwrap_err().starts_with("ball.wall_restitution"));
}
