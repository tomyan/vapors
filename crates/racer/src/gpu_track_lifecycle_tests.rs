//! GPU tier: the track surface's edges. A paused scene that changes (a
//! restart, the end of a camera transition) renders once and the app
//! waits; a render that fails ends the racer with its failure; dropping
//! the racer drops its world's renderer and frees what it made.
use crate::app::Racer;
use crate::gpu_frames::{drive, frame_at, frames, idle, settle};
use std::time::Duration;
use vapors_app::{SurfaceError, SurfaceFrame, SurfaceRenderer, SurfaceSize};
use vapors_gpu_harness::gpu;
use vapors_platform::{Instant, Next};
use vapors_scene3d_gpu::Resource;

/// Mounts and resizes, then refuses every render.
struct FailingTrack;
impl SurfaceRenderer for FailingTrack {
    fn mount(&mut self, _: &wgpu::Device) -> Result<(), SurfaceError> {
        Ok(())
    }
    fn resize(&mut self, _: &wgpu::Device, _: SurfaceSize) -> Result<(), SurfaceError> {
        Ok(())
    }
    fn render(&mut self, _: SurfaceFrame<'_>) -> Result<(), SurfaceError> {
        Err("the test track refuses to render".into())
    }
}

#[test]
fn gpu_restarting_while_paused_renders_the_track_once_and_waits() {
    // Given: raced (the car moved), paused, idle
    let gpu = gpu();
    let mut racer = idle(&gpu);
    drive(&mut racer);
    frames(&mut racer, &gpu, 2);
    racer.command("toggle").expect("pauses");
    settle(&mut racer, &gpu);
    let before = racer.track.renders();

    // When
    racer.command("restart").expect("restarts");
    let after = frames(&mut racer, &gpu, 3);

    // Then: the first frame draws the HUD's readouts and the track
    assert_eq!(after, [Next::Wait; 3]);
    assert_eq!(racer.track.renders() - before, 1);
}

#[test]
fn gpu_the_end_of_a_paused_camera_transition_renders_once_and_waits() {
    // Given: idle; a camera toggle moves the camera while paused
    let gpu = gpu();
    let mut racer = idle(&gpu);
    let before = racer.track.renders();
    racer.command("camera").expect("toggles the camera");

    // When: 50 ms frames (the camera's step cap) until one asks for none
    let mut now = Instant::now();
    let mut drawn = 0;
    let settled = (0..200).any(|_| {
        now += Duration::from_millis(50);
        drawn += 1;
        frame_at(&mut racer, &gpu, now).0 == Next::Wait
    });
    let moved = racer.track.renders() - before;
    let idle_frames = frames(&mut racer, &gpu, 2);

    // Then: every frame the camera moved in rendered, the last (the
    // camera at rest) included; not the first, which comes 0 s after the
    // toggle, so the camera has not moved yet
    assert!(settled, "the camera still moves after 200 frames");
    assert!(!racer.run.camera.moving);
    assert_eq!(moved, drawn - 1);
    assert_eq!(idle_frames, [Next::Wait; 2]);
    assert_eq!(racer.track.renders() - before, drawn - 1);
}

#[test]
fn gpu_a_track_render_failure_ends_the_racer_with_the_failure() {
    // Given: the track's renderer swapped for one that fails
    let gpu = gpu();
    let mut racer = Racer::new();
    racer
        .app
        .unmount("track")
        .expect("the racer mounted its track");
    let failing = Box::new(FailingTrack);
    racer.app.mount("track", failing).expect("mounts");

    // When
    let (next, _) = frame_at(&mut racer, &gpu, Instant::now());

    // Then: `run` reads `failure` after the loop and returns it
    assert_eq!(next, Next::Exit);
    let failure = racer.failure().get().unwrap_or_default();
    assert!(
        failure.contains("the test track refuses to render"),
        "{failure:?}"
    );
    assert_eq!(racer.faults.error, Some(failure));
    assert!(!racer.framed, "a frame that exited drew nothing");
}

#[test]
fn gpu_dropping_the_racer_drops_its_worlds_renderer_and_frees_all_it_made() {
    // Given: a racer that has drawn its world
    let gpu = gpu();
    let racer = idle(&gpu);
    let log = racer.track.log();
    let made: Vec<u64> = Resource::ALL.iter().map(|&kind| log.made(kind)).collect();

    // When
    drop(racer);

    // Then
    assert!(log.dropped());
    assert!(made.iter().all(|&count| count > 0), "{made:?}");
    let freed: Vec<u64> = Resource::ALL.iter().map(|&kind| log.freed(kind)).collect();
    assert_eq!(freed, made);
}
