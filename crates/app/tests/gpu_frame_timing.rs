//! GPU tier: an `ElementsApp` times its frames on the GPU when the
//! device has timestamp queries: frame n's time, a pair per render pass
//! and one per `gpu-surface` render, is read back by a later frame.
//! Without them its GPU time is `Unsupported`, never an estimate.

#[path = "support/surface.rs"]
mod support;

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use support::{RED, SIZE, Solid, document};
use vapors_app::{ElementsApp, GpuReading, GpuTime};
use vapors_gpu_harness::{Gpu, gpu, gpu_with};
use vapors_platform::{App, Frame, Instant};
use vapors_renderer::clear;
use vapors_scene::Color;

const TRACK: &str = ".track { width: 16px; height: 16px; }";

/// A GPU with timestamp queries, refused by name without them.
fn timed_gpu() -> Gpu {
    let gpu = gpu_with(wgpu::Features::TIMESTAMP_QUERY);
    let features = gpu.device.features();
    assert!(
        features.contains(wgpu::Features::TIMESTAMP_QUERY),
        "the adapter has no TIMESTAMP_QUERY ({features:?}); run with VAPORS_GPU_ADAPTER \
         naming one that has, as ANV and llvmpipe do"
    );
    gpu
}

fn app_of(css: &str, elements: &[(Option<usize>, &str, &str)]) -> ElementsApp {
    ElementsApp::new(Rc::new(RefCell::new(document(css, elements))))
}

/// Frame `index` of `app`, then a wait for the GPU to finish it.
fn frame(app: &mut ElementsApp, gpu: &Gpu, index: u64) {
    let texture = clear(&gpu.device, &gpu.queue, SIZE, SIZE, Color::default());
    app.frame(&Frame {
        adapter: &gpu.adapter,
        device: &gpu.device,
        queue: &gpu.queue,
        texture: &texture,
        index,
        scale_factor: 1.0,
        now: Instant::now(),
    });
    let wait = wgpu::PollType::Wait {
        submission_index: None,
        timeout: Some(Duration::from_secs(10)),
    };
    gpu.device.poll(wait).expect("the GPU finished within 10 s");
    assert_eq!(app.failure().get(), None);
}

fn read(time: GpuTime) -> GpuReading {
    match time {
        GpuTime::Read(reading) => reading,
        other => panic!("frame 0 was not read back by frame 1: {other:?}"),
    }
}

#[test]
fn gpu_without_timestamp_queries_the_gpu_time_is_unsupported_once_a_frame_paints() {
    // Given
    let (mut app, gpu) = (app_of("", &[]), gpu());
    assert_eq!(app.frame_timing().gpu, GpuTime::Pending);

    // When
    frame(&mut app, &gpu, 0);
    frame(&mut app, &gpu, 1);

    // Then
    assert_eq!(app.frame_timing().gpu, GpuTime::Unsupported);
}

#[test]
fn gpu_a_frame_is_read_back_by_the_next_with_a_pair_per_render_pass() {
    // Given
    let (mut app, gpu) = (app_of("", &[]), timed_gpu());
    frame(&mut app, &gpu, 0);
    let passes = app.render_passes();

    // When
    frame(&mut app, &gpu, 1);

    // Then
    let reading = read(app.frame_timing().gpu);
    assert_eq!(reading.frame, 0);
    assert_eq!(u64::from(reading.pairs), passes);
    let time = reading.time.expect("a measured time");
    assert!(
        time > Duration::ZERO && time < Duration::from_secs(1),
        "{time:?}"
    );
}

#[test]
fn gpu_a_surface_render_adds_a_pair_of_its_own() {
    // Given
    let mut app = app_of(TRACK, &[(None, "gpu-surface", "track")]);
    let gpu = timed_gpu();
    let (solid, _log) = Solid::new(RED);
    app.mount("track", Box::new(solid)).expect("mounts");
    frame(&mut app, &gpu, 0);
    let passes = app.render_passes();

    // When
    frame(&mut app, &gpu, 1);

    // Then
    let reading = read(app.frame_timing().gpu);
    assert_eq!((reading.frame, u64::from(reading.pairs)), (0, passes + 1));
    assert!(reading.time.is_ok(), "{reading:?}");
}
