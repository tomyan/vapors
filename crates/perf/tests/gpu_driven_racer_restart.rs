//! GPU tier: the driven racer through three laps and its restart, on a
//! small target with 50 ms frames (the racer's largest step), so the
//! finish comes in about 500 frames rather than the run's 1,533.

use std::time::Duration;

use vapors_gpu_harness::{Gpu, gpu_with};
use vapors_perf::gpu_time::{FEATURES, wait};
use vapors_perf::racer_driven::DrivenRacer;
use vapors_perf::run::Workload;
use vapors_platform::{Frame, Instant};
use vapors_renderer::clear;
use vapors_scene::Color;

/// The racer's largest step per frame (`advance_elapsed`).
const STEP: Duration = Duration::from_millis(50);

/// One checked frame at `now`, the script's input before it.
fn frame(driven: &mut DrivenRacer, gpu: &Gpu, texture: &wgpu::Texture, index: usize, now: Instant) {
    driven.change(index);
    driven.ready().expect("the script's input took");
    let frame = Frame {
        adapter: &gpu.adapter,
        device: &gpu.device,
        queue: &gpu.queue,
        texture,
        index: index as u64,
        scale_factor: 1.0,
        now,
    };
    let next = driven.app().frame(&frame);
    driven
        .check(next)
        .unwrap_or_else(|error| panic!("frame {index}: {error}"));
    wait(&gpu.device, Duration::from_secs(10)).expect("the GPU finished");
}

#[test]
fn gpu_the_driven_racer_finishes_three_laps_restarts_and_races_on() {
    // Given
    let gpu = gpu_with(FEATURES);
    let texture = clear(&gpu.device, &gpu.queue, 320, 240, Color::default());
    let (mut driven, mut now) = (DrivenRacer::new(), Instant::now());

    // When: frames until three laps, then five more
    let mut finished = None;
    for index in 0..900 {
        now += STEP;
        frame(&mut driven, &gpu, &texture, index, now);
        if finished.is_none() && driven.racer().race().progress.laps == 3 {
            finished = Some(index);
        }
        if finished.is_some_and(|at| index == at + 5) {
            break;
        }
    }

    // Then: the race started again and runs
    let finished = finished.expect("three laps within 900 frames of 50 ms");
    let race = driven.racer().race();
    assert_eq!(race.progress.laps, 0, "restarted after frame {finished}");
    assert!(race.elapsed > 0.0 && !driven.racer().paused());
}
