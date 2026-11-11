//! GPU tier: a timed frame writes a timestamp pair around each render
//! pass and timed section, read back after the GPU finishes; a frame
//! with no time says why; a device without timestamp queries has no
//! timer. The reference adapters (ANV, lavapipe) have them.

#[path = "support/frame.rs"]
mod frame;

use std::time::Duration;

use frame::{quad, square, target};
use vapors_gpu_harness::{Gpu, gpu, gpu_with};
use vapors_renderer::{FrameEncoder, GpuMissing, GpuReading, GpuTime, GpuTimer, QuadPipeline};

const RED: [u8; 4] = [255, 0, 0, 255];

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

/// Draws a quad on each of `views` through `encoder`: one pass each.
fn draw_on(gpu: &Gpu, encoder: &FrameEncoder, views: &[&wgpu::TextureView]) {
    let mut pipeline = QuadPipeline::new(&gpu.device, wgpu::TextureFormat::Rgba8Unorm);
    for view in views {
        pipeline.draw(
            &target(gpu, encoder, view, 4),
            &[quad([0.0, 0.0, 4.0, 4.0], RED)],
        );
    }
}

/// Waits for the GPU, then reads back what it finished.
fn read_back(gpu: &Gpu, timer: &mut GpuTimer) -> GpuTime {
    let wait = wgpu::PollType::Wait {
        submission_index: None,
        timeout: Some(Duration::from_secs(10)),
    };
    gpu.device.poll(wait).expect("the GPU finished within 10 s");
    timer.poll();
    timer.time()
}

#[test]
fn gpu_a_timed_frame_reads_back_one_pair_per_render_pass_once_the_gpu_finishes() {
    // Given
    let gpu = timed_gpu();
    let mut timer = GpuTimer::new(&gpu.device, &gpu.queue).expect("timestamp queries");
    let ((_, first), (_, second)) = (square(&gpu, 4), square(&gpu, 4));
    let encoder = FrameEncoder::timed(&gpu.device, timer.frame(5));

    // When
    draw_on(&gpu, &encoder, &[&first, &second]);
    encoder.submit(&gpu.queue).expect("no draw refused");
    let before = timer.time();
    let after = read_back(&gpu, &mut timer);

    // Then
    assert_eq!(before, GpuTime::Pending);
    let GpuTime::Read(GpuReading {
        frame: 5,
        pairs: 2,
        time: Ok(time),
    }) = after
    else {
        panic!("{after:?}");
    };
    assert!(time > Duration::ZERO, "{time:?}");
}

#[test]
fn gpu_a_device_without_timestamp_queries_has_no_timer() {
    // Given
    let gpu = gpu();

    // When
    let timer = GpuTimer::new(&gpu.device, &gpu.queue);

    // Then
    assert!(timer.is_none());
}

#[test]
fn gpu_a_frame_dropped_unsubmitted_reads_back_as_not_submitted() {
    // Given
    let gpu = timed_gpu();
    let mut timer = GpuTimer::new(&gpu.device, &gpu.queue).expect("timestamp queries");
    let (_, view) = square(&gpu, 4);

    // When
    let encoder = FrameEncoder::timed(&gpu.device, timer.frame(0));
    draw_on(&gpu, &encoder, &[&view]);
    drop(encoder);
    let time = read_back(&gpu, &mut timer);

    // Then
    let not_submitted = GpuReading {
        frame: 0,
        pairs: 1,
        time: Err(GpuMissing::NotSubmitted),
    };
    assert_eq!(time, GpuTime::Read(not_submitted));
}

#[test]
fn gpu_a_frame_of_more_passes_than_its_pairs_reads_back_as_over_capacity() {
    // Given
    let gpu = timed_gpu();
    let mut timer = GpuTimer::with_capacity(&gpu.device, &gpu.queue, 1).expect("timestamps");
    let ((_, first), (_, second)) = (square(&gpu, 4), square(&gpu, 4));

    // When
    let encoder = FrameEncoder::timed(&gpu.device, timer.frame(0));
    draw_on(&gpu, &encoder, &[&first, &second]);
    encoder.submit(&gpu.queue).expect("no draw refused");
    let time = read_back(&gpu, &mut timer);

    // Then
    let over = GpuMissing::OverCapacity {
        pairs: 2,
        capacity: 1,
    };
    let GpuTime::Read(GpuReading {
        time: Err(missing), ..
    }) = time
    else {
        panic!("{time:?}");
    };
    assert_eq!(missing, over);
}

#[test]
fn gpu_a_timed_section_in_the_callers_encoder_adds_one_pair() {
    // Given
    let gpu = timed_gpu();
    let mut timer = GpuTimer::new(&gpu.device, &gpu.queue).expect("timestamp queries");
    let ((_, frame_view), (_, section_view)) = (square(&gpu, 4), square(&gpu, 4));
    let encoder = FrameEncoder::timed(&gpu.device, timer.frame(0));
    let mut own = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

    // When
    encoder.time_section(&mut own, &section_view, |_| ());
    encoder.submit_section(&gpu.queue, own.finish());
    draw_on(&gpu, &encoder, &[&frame_view]);
    encoder.submit(&gpu.queue).expect("no draw refused");
    let time = read_back(&gpu, &mut timer);

    // Then
    let GpuTime::Read(GpuReading {
        pairs: 2,
        time: Ok(_),
        ..
    }) = time
    else {
        panic!("{time:?}");
    };
}

#[test]
fn gpu_a_timed_section_left_unsubmitted_reads_back_as_section_not_submitted() {
    // Given
    let gpu = timed_gpu();
    let mut timer = GpuTimer::new(&gpu.device, &gpu.queue).expect("timestamp queries");
    let ((_, frame_view), (_, section_view)) = (square(&gpu, 4), square(&gpu, 4));
    let encoder = FrameEncoder::timed(&gpu.device, timer.frame(0));
    let mut own = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

    // When: the section's encoder is dropped, as a failed render drops it
    encoder.time_section(&mut own, &section_view, |_| ());
    drop(own);
    draw_on(&gpu, &encoder, &[&frame_view]);
    encoder.submit(&gpu.queue).expect("no draw refused");
    let time = read_back(&gpu, &mut timer);

    // Then
    let GpuTime::Read(GpuReading {
        time: Err(missing), ..
    }) = time
    else {
        panic!("{time:?}");
    };
    assert_eq!(missing, GpuMissing::SectionNotSubmitted { sections: 1 });
}
