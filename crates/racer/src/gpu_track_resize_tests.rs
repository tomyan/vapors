//! GPU tier: the window resized and its device scale changed mid-race.
//! The track's renderer reallocates its depth texture once per size
//! change and renders at the new size from that frame; the HUD lays out
//! for the new size; at 2x the backing is rebuilt at twice the device
//! size for the same CSS size, and the track shows the same colours
//! (docs/gpu-surface.md, 3D slice 1c PR 6).
use crate::app::Racer;
use crate::gpu_frames::{WINDOW, frame_in, hold_button, idle, pixel_in, track_alone_in};
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
const DOUBLE: SurfaceSize = SurfaceSize {
    width: WINDOW.width * 2,
    height: WINDOW.height * 2,
    scale: 2.0,
};

/// A racer racing for two frames in the default window.
fn racing(gpu: &Gpu) -> (Racer, Instant) {
    let mut racer = idle(gpu);
    racer.command("toggle").expect("drives");
    let now = Instant::now();
    for _ in 0..2 {
        drawn_in(&mut racer, gpu, WINDOW, now);
    }
    (racer, now)
}

/// One frame in `window` at `now` that must not fail and must ask for
/// the next; its pixels.
fn drawn_in(racer: &mut Racer, gpu: &Gpu, window: SurfaceSize, now: Instant) -> Vec<u8> {
    let (next, rgba) = frame_in(racer, gpu, window, now);
    assert_eq!(racer.app.failure().get(), None, "the frame failed");
    assert_eq!(next, Next::Continue, "a racing frame asks for the next");
    rgba
}

#[test]
fn gpu_a_window_resized_mid_race_reallocates_the_depth_texture_once_and_renders_at_the_new_size() {
    // Given
    let gpu = gpu();
    let (mut racer, now) = racing(&gpu);
    let resizes = racer.track.log().resizes.get();

    // When: three frames at the new size; each renders at it
    let mut rgba = Vec::new();
    for _ in 0..3 {
        rgba = drawn_in(&mut racer, &gpu, NARROW, now);
        assert_eq!(racer.track.log().rendered.get(), Some(NARROW));
    }

    // Then
    let log = racer.track.log();
    assert_eq!(log.resizes.get() - resizes, 1, "one resize per size change");
    assert_eq!(log.depth.get(), Some([NARROW.width, NARROW.height]));
    let centre = [350.0, 300.0];
    let alone = track_alone_in(&racer, &gpu, NARROW);
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
fn gpu_a_device_scale_change_mid_race_rebuilds_the_backing_at_the_new_device_size() {
    // Given: racing at 1x
    let gpu = gpu();
    let (mut racer, now) = racing(&gpu);
    let resizes = racer.track.log().resizes.get();

    // When: 2x, then 1x again
    drawn_in(&mut racer, &gpu, DOUBLE, now);
    let doubled = racer.track.log().rendered.get().expect("rendered");
    let surfaces_doubled = racer.app.frame_counts().surfaces.draw_calls;
    drawn_in(&mut racer, &gpu, WINDOW, now);

    // Then
    let log = racer.track.log();
    assert_eq!(doubled, DOUBLE);
    assert_eq!(doubled.logical(), WINDOW.logical());
    assert_eq!(
        log.resizes.get() - resizes,
        2,
        "one resize per scale change"
    );
    assert_eq!(log.depth.get(), Some([WINDOW.width, WINDOW.height]));
    assert_eq!(log.rendered.get(), Some(WINDOW));
    assert_eq!(surfaces_doubled, 1, "one composition draw at 2x");
    assert_eq!(racer.app.frame_counts().surfaces.draw_calls, 1);
}

#[test]
fn gpu_the_track_shows_the_same_colour_at_one_and_two_times_scale() {
    // Given: racing, the clock held at `now`, so the scene does not move
    let gpu = gpu();
    let (mut racer, now) = racing(&gpu);
    let [x, y, width, height] = hold_button(&racer, "gas");
    let open = [x - width * 4.0, y - height * 4.0];

    // When
    let single = drawn_in(&mut racer, &gpu, WINDOW, now);
    let double = drawn_in(&mut racer, &gpu, DOUBLE, now);

    // Then: the CSS point `open` is device pixel `open` at 1x, twice it at 2x
    let at_double = [open[0] * 2.0, open[1] * 2.0];
    let colour = pixel_in(&single, WINDOW.width, open);
    assert_ne!(colour, [255, 0, 255, 255], "the track drew nothing there");
    assert_eq!(pixel_in(&double, DOUBLE.width, at_double), colour);
}
