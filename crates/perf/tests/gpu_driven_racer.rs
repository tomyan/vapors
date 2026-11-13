//! GPU tier: the 60 s run's driven racer, without timing: the car
//! moves under the script, and a scripted camera switch goes through
//! the HUD's button and leaves Space to the race.

use vapors_gpu_harness::gpu_with;
use vapors_perf::gpu_time::FEATURES;
use vapors_perf::racer_driven::DrivenRacer;
use vapors_perf::racer_input::key;
use vapors_perf::run::{Settings, Workload};
use vapors_perf::sampler::Sampler;
use vapors_racer::model::Race;

#[test]
fn gpu_the_driven_racer_moves_and_every_frame_asks_for_the_next() {
    // Given: 0.96 s of the frame clock
    let mut driven = DrivenRacer::new();
    let gpu = gpu_with(FEATURES);

    // When: the workload's check refuses a frame that asks for none
    let samples = Sampler::new(&gpu)
        .sample(
            &mut driven,
            Settings {
                warmup: 0,
                runs: 60,
            },
        )
        .expect("every frame ran and asked for the next");

    // Then
    assert_eq!(samples.frame.len(), 60);
    let race = driven.racer().race();
    assert!(race.speed > 50.0, "speed {}", race.speed);
    assert_ne!(race.position, Race::default().position);
}

#[test]
fn gpu_a_scripted_camera_switch_clicks_the_button_and_space_still_reaches_the_race() {
    // Given: one frame drawn, so the HUD is laid out
    let mut driven = DrivenRacer::new();
    let gpu = gpu_with(FEATURES);
    let one = Settings { warmup: 0, runs: 1 };
    Sampler::new(&gpu)
        .sample(&mut driven, one)
        .expect("a frame");

    // When: the input before the frame at 10 s, then Space
    driven.change(625);
    driven.ready().expect("the switch happened");
    let follows = driven.racer().camera().follow;
    key(driven.app(), "Space", true);
    key(driven.app(), "Space", false);

    // Then: Space paused the race and left the camera alone
    assert!(follows, "the camera follows after the switch");
    assert!(driven.racer().paused(), "Space reached the race");
    assert!(
        driven.racer().camera().follow,
        "Space did not press the camera button"
    );
}
