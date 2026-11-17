//! GPU tier: the window resized and its device scale changed mid-race.
//! The world renders at the new size from that frame, as it renders
//! alone at that size; the HUD lays out for the new size; at 2x it
//! renders on each frame with one composition draw, and the world shows
//! the same colours (`gpu_track_scale_tests.rs`; docs/gpu-surface.md,
//! 3D slice 1c PR 6). The scene renderer's own resize (its depth and 4x
//! textures per size) is pinned in `vapors-scene3d-gpu`.
use crate::app::Racer;
use crate::gpu_frames::{WINDOW, drive, frame_in, hold_button, idle, pixel_in, world_alone_in};
use vapors_app::SurfaceSize;
use vapors_gpu_harness::{Gpu, gpu};
use vapors_platform::{Instant, Next};

/// Narrower than the HUD's compact breakpoint (760 CSS pixels).
const NARROW: SurfaceSize = SurfaceSize {
    width: 700,
    height: 600,
    scale: 1.0,
};
/// The window at device scale 2: the same CSS size.
pub(crate) const DOUBLE: SurfaceSize = SurfaceSize {
    width: WINDOW.width * 2,
    height: WINDOW.height * 2,
    scale: 2.0,
};

/// A racer racing, the car moving, for two frames in the default window.
pub(crate) fn racing(gpu: &Gpu) -> (Racer, Instant) {
    let mut racer = idle(gpu);
    drive(&mut racer);
    let now = Instant::now();
    for _ in 0..2 {
        drawn_in(&mut racer, gpu, WINDOW, now);
    }
    (racer, now)
}

/// One frame in `window` at `now` that must not fail and must ask for
/// the next; its pixels.
pub(crate) fn drawn_in(racer: &mut Racer, gpu: &Gpu, window: SurfaceSize, now: Instant) -> Vec<u8> {
    let (next, rgba) = frame_in(racer, gpu, window, now);
    assert_eq!(racer.app.failure().get(), None, "the frame failed");
    assert_eq!(
        next,
        Next::Continue,
        "Continue was expected: the race runs in every case here, so the frame must ask for the next; got {next:?}"
    );
    rgba
}

#[test]
fn gpu_a_window_resized_mid_race_renders_the_world_once_at_the_new_size() {
    // Given
    let gpu = gpu();
    let (mut racer, now) = racing(&gpu);
    let before = racer.track.renders();

    // When: three frames at the new size, the clock held, so only the
    // size changes
    let mut rgba = Vec::new();
    for _ in 0..3 {
        rgba = drawn_in(&mut racer, &gpu, NARROW, now);
    }

    // Then: the size change rendered, once; the frames after it show it,
    // as the world alone renders at that size where no control covers it
    // (a pass at the wrong size is refused by wgpu, failing the frame)
    assert_eq!(racer.track.renders() - before, 1);
    let centre = [350.0, 300.0];
    let alone = world_alone_in(&racer, &gpu, NARROW);
    assert_ne!(pixel_in(&alone, NARROW.width, centre), [255, 0, 255, 255]);
    assert_eq!(
        pixel_in(&rgba, NARROW.width, centre),
        pixel_in(&alone, NARROW.width, centre)
    );
}

#[test]
fn gpu_a_window_resized_mid_race_lays_the_hud_out_for_the_new_size() {
    // Given
    let gpu = gpu();
    let (mut racer, now) = racing(&gpu);
    let wide = hold_button(&racer, "gas");

    // When
    drawn_in(&mut racer, &gpu, NARROW, now);

    // Then: the compact layout moves the gas button inside the window
    let [x, y, width, height] = hold_button(&racer, "gas");
    assert!(racer.hud.compact.get());
    assert_ne!([x, y], [wide[0], wide[1]]);
    assert!(x + width <= 700.0 && y + height <= 600.0, "{x} {y}");
}

#[test]
fn gpu_a_device_scale_change_mid_race_renders_on_each_frame_with_one_composition() {
    // Given: racing at 1x
    let gpu = gpu();
    let (mut racer, now) = racing(&gpu);
    let before = racer.track.renders();

    // When: 2x, then 1x again
    drawn_in(&mut racer, &gpu, DOUBLE, now);
    let surfaces_doubled = racer.app.frame_counts().surfaces.draw_calls;
    drawn_in(&mut racer, &gpu, WINDOW, now);

    // Then: the world rendered at each; the page composed it once at each
    assert_eq!(racer.track.renders() - before, 2);
    assert_eq!(DOUBLE.logical(), WINDOW.logical());
    assert_eq!(
        surfaces_doubled,
        racer.app.frame_counts().surfaces.draw_calls
    );
}
