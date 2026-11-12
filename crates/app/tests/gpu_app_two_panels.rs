//! GPU tier: two panels side by side, each holding a `gpu-surface` with
//! its own renderer, animated through its own `SurfaceHandle`
//! (docs/gpu-surface.md, 3D slice 1c PR 4). Both render once per frame
//! while both animate; one stopped renders nothing while the other
//! renders every frame; a renderer that fills a backing larger than its
//! panel paints only inside the panel, and under UI painted after it.
//! Resizing and `FrameCounts` are in `gpu_app_two_panels_resize.rs`.
#[path = "support/panels.rs"]
mod panels;
#[path = "support/surface.rs"]
mod support;

use panels::{OVERSIZED, Panels};
use support::{BLUE, RED, WHITE, pixel};
use vapors_platform::Next;

const GREEN: [u8; 4] = [0, 255, 0, 255];

#[test]
fn gpu_two_animating_panels_each_render_once_per_frame() {
    // Given
    let mut panels = Panels::animating(OVERSIZED);
    let before = panels.renders();

    // When
    let (asked, _) = panels.frames(3);

    // Then
    assert_eq!(asked, [Next::Continue; 3]);
    assert_eq!(panels.renders(), (before.0 + 3, before.1 + 3));
}

#[test]
fn gpu_a_stopped_panel_renders_nothing_while_the_other_renders_every_frame() {
    // Given
    let mut panels = Panels::animating(OVERSIZED);
    panels.frames(2);

    // When
    panels.track.0.stop_animation().expect("mounted");
    let before = panels.renders();
    let (asked, _) = panels.frames(3);

    // Then
    assert_eq!(asked, [Next::Continue; 3]);
    assert_eq!(panels.renders(), (before.0, before.1 + 3));
}

#[test]
fn gpu_a_surface_larger_than_its_panel_paints_only_inside_the_panel() {
    // Given
    let mut panels = Panels::new(OVERSIZED, false);

    // When
    let (_, rgba) = panels.frames(1);

    // Then
    let at = |points: [(u32, u32); 2]| points.map(|(x, y)| pixel(&rgba, x, y));
    assert_eq!(at([(2, 2), (13, 13)]), [RED, RED], "inside the left panel");
    assert_eq!(at([(18, 2), (29, 13)]), [BLUE, BLUE], "inside the right");
    assert_eq!(at([(14, 8), (8, 14)]), [WHITE, WHITE], "past the left");
    assert_eq!(at([(17, 8), (30, 8)]), [WHITE, WHITE], "beside the right");
    assert_eq!(at([(24, 14), (1, 1)]), [WHITE, WHITE], "under, before");
    assert_eq!(
        at([(1, 8), (8, 1)]),
        [WHITE, WHITE],
        "left of, above the left"
    );
    assert_eq!(
        at([(17, 2), (14, 2)]),
        [WHITE, WHITE],
        "left of the right, right of the left"
    );
}

#[test]
fn gpu_ui_painted_after_both_panels_stays_over_their_surfaces() {
    // Given
    let mut panels = Panels::new(OVERSIZED, true);

    // When
    let (_, rgba) = panels.frames(1);

    // Then
    let on_bar = [(4, 6), (12, 7), (20, 6), (28, 7)].map(|(x, y)| pixel(&rgba, x, y));
    assert_eq!(on_bar, [GREEN; 4]);
    assert_eq!([pixel(&rgba, 4, 9), pixel(&rgba, 20, 9)], [RED, BLUE]);
}
