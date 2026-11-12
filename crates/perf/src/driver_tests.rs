//! The scripted driver's controls, and its schedule against simulated
//! time with the race model stepped as the racer steps it.

use super::{RUN_FRAMES, Scripted, controls, restart_due, scripted};
use crate::circuit::{BEND_STARTS, point_at};
use vapors_racer::model::{ROAD, Race, Settings, road_distance};

/// The racer's fixed step (`app/drawing.rs`, `step_fixed`).
const STEP: f32 = 1.0 / 120.0;

/// A race at `distance` along the centre line, heading along it.
fn race_at(distance: f32, speed: f32) -> Race {
    let [from, to] = [point_at(distance), point_at(distance + 1.0)];
    let mut race = Race::default();
    race.position = from;
    race.heading = (to[1] - from[1]).atan2(to[0] - from[0]);
    race.speed = speed;
    race
}

#[test]
fn a_car_on_the_line_heading_along_it_steers_straight_and_holds_the_gas() {
    let held = controls(&race_at(20.0, 100.0));
    assert_eq!([held.throttle, held.brake, held.steer], [1.0, 0.0, 0.0]);
}

#[test]
fn a_car_left_of_the_line_steers_right_and_right_of_it_steers_left() {
    // Given: on the top straight, heading +X; Y grows downward, to the right
    let mut left = race_at(20.0, 100.0);
    left.position[1] -= 10.0;
    let mut right = race_at(20.0, 100.0);
    right.position[1] += 10.0;

    // Then: steering right turns clockwise (`Race::drive`)
    assert_eq!(controls(&left).steer, 1.0);
    assert_eq!(controls(&right).steer, -1.0);
}

#[test]
fn a_fast_car_brakes_just_before_a_bend_and_nowhere_else() {
    for bend in BEND_STARTS {
        let braking = controls(&race_at(bend - 20.0, 200.0));
        assert_eq!([braking.throttle, braking.brake], [0.0, 1.0], "{bend}");
        let slower = controls(&race_at(bend - 20.0, 150.0));
        assert_eq!(slower.brake, 0.0, "{bend} at 150");
        let earlier = controls(&race_at(bend - 120.0, 200.0));
        assert_eq!(earlier.brake, 0.0, "{bend}, 120 before");
    }
}

/// What a 60 s run did, frame by frame.
#[derive(Default)]
struct Run {
    scripted: Vec<(usize, Scripted)>,
    restarts: Vec<usize>,
    furthest_off_line: f32,
}

/// The run as the racer steps it: before each frame the driver's
/// restart and controls; each frame's elapsed time added to the fixed
/// step's remainder, 0 on the first frame after a start (`resume` clears
/// the last frame's time; `pause` clears the remainder).
fn drive() -> Run {
    let (mut race, mut run, mut remainder, mut started) =
        (Race::default(), Run::default(), 0.0, true);
    for frame in 0..RUN_FRAMES {
        run.scripted
            .extend(scripted(frame).into_iter().map(|s| (frame, s)));
        if restart_due(&race) {
            assert_eq!(race.progress.laps, 3, "a restart only after three laps");
            (race, remainder, started) = (Race::default(), 0.0, true);
            run.restarts.push(frame);
        }
        remainder += if std::mem::take(&mut started) {
            0.0
        } else {
            0.016
        };
        let held = controls(&race);
        while remainder >= STEP {
            race.step(STEP, held, Settings::default())
                .expect("a valid step");
            remainder -= STEP;
        }
        run.furthest_off_line = run.furthest_off_line.max(road_distance(race.position));
    }
    run
}

#[test]
fn the_schedule_switches_camera_every_10_s_and_livery_at_30_s() {
    let run = drive();
    let camera = |frame| (frame, Scripted::CameraSwitch);
    let expected = vec![
        camera(625),
        camera(1250),
        camera(1875),
        (1875, Scripted::Livery),
        camera(2500),
        camera(3125),
    ];
    assert_eq!(run.scripted, expected);
}

#[test]
fn the_driver_finishes_three_laps_on_the_road_and_the_race_restarts() {
    let run = drive();
    // three laps take 24.5 s of the race clock (7.7 s a lap)
    assert_eq!(
        run.restarts.len(),
        2,
        "restarts at frames {:?}",
        run.restarts
    );
    assert!(
        run.furthest_off_line < ROAD,
        "{} off the line",
        run.furthest_off_line
    );
}
