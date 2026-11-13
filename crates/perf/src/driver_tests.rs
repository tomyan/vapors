//! The scripted driver's controls, and its schedule against simulated
//! time with the race model stepped as the racer steps it.

use std::collections::BTreeSet;
use std::time::Duration;

use super::{RUN_FRAMES, Scripted, controls, restart_due, scripted};
use crate::circuit::{BEND_STARTS, point_at};
use crate::sampler::INTERVAL;
use vapors_racer::model::{Controls, ROAD, Race, Settings, road_distance};

/// The racer's fixed step (`app/drawing.rs`, `step_fixed`), a literal
/// there with no constant to read.
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
    /// Each lap that braked, as (races finished before it, its lap).
    braking_laps: BTreeSet<(usize, u8)>,
    /// The steering values the driver held.
    steering: BTreeSet<i8>,
}

/// The race model stepped as the racer steps it: each frame's elapsed
/// time added to the fixed step's remainder, 0 on the first frame after
/// a start (`resume` clears the last frame's time; `pause` clears the
/// remainder).
struct Stepped {
    race: Race,
    remainder: f32,
    started: bool,
}

impl Stepped {
    fn new() -> Self {
        Self {
            race: Race::default(),
            remainder: 0.0,
            started: true,
        }
    }

    /// One frame under `held`.
    fn frame(&mut self, held: Controls) {
        self.remainder += if std::mem::take(&mut self.started) {
            0.0
        } else {
            INTERVAL.as_secs_f32()
        };
        while self.remainder >= STEP {
            self.race
                .step(STEP, held, Settings::default())
                .expect("a valid step");
            self.remainder -= STEP;
        }
    }
}

/// The run: before each frame the script's changes, its restart and
/// its controls, then the frame.
fn drive() -> Run {
    let (mut stepped, mut run) = (Stepped::new(), Run::default());
    for frame in 0..RUN_FRAMES {
        run.scripted
            .extend(scripted(frame).into_iter().map(|s| (frame, s)));
        if restart_due(&stepped.race) {
            assert_eq!(
                stepped.race.progress.laps, 3,
                "a restart only after three laps"
            );
            stepped = Stepped::new();
            run.restarts.push(frame);
        }
        let held = controls(&stepped.race);
        run.note(&stepped.race, held);
        stepped.frame(held);
        run.furthest_off_line = run
            .furthest_off_line
            .max(road_distance(stepped.race.position));
    }
    run
}

impl Run {
    #[allow(clippy::cast_possible_truncation)] // -1, 0 or 1
    fn note(&mut self, race: &Race, held: Controls) {
        if held.brake > 0.0 {
            self.braking_laps
                .insert((self.restarts.len(), race.progress.laps));
        }
        self.steering.insert(held.steer as i8);
    }
}

#[test]
fn the_schedule_names_the_camera_every_10_s_and_changes_livery_at_30_s() {
    let run = drive();
    let camera = |frame, follow| (frame, Scripted::Camera { follow });
    let expected = vec![
        camera(625, true),
        camera(1250, false),
        camera(1875, true),
        (1875, Scripted::Livery),
        camera(2500, false),
        camera(3125, true),
    ];
    assert_eq!(run.scripted, expected);
}

#[test]
fn the_run_is_60_s_of_the_frame_clock() {
    assert_eq!(
        INTERVAL * u32::try_from(RUN_FRAMES).expect("small"),
        Duration::from_secs(60)
    );
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

#[test]
fn every_finished_lap_brakes_and_the_driver_steers_both_ways() {
    let run = drive();
    for lap in (0..run.restarts.len()).flat_map(|race| (0..3).map(move |lap| (race, lap))) {
        assert!(
            run.braking_laps.contains(&lap),
            "lap {lap:?} never braked: {:?}",
            run.braking_laps
        );
    }
    assert_eq!(run.steering, BTreeSet::from([-1, 0, 1]));
}
