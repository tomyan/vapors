//! GPU tier: a panel resized by layout resizes its surface once and
//! renders at the new size; `FrameCounts` counts one composition per
//! panel (docs/gpu-surface.md, 3D slice 1c PR 4).

#[path = "support/panels.rs"]
mod panels;
#[path = "support/surface.rs"]
mod support;

use panels::{FLEXIBLE, OVERSIZED, Panels};
use support::{BLUE, RED, pixel};
use vapors_app::SurfaceSize;
use vapors_dom::Dom;

#[test]
fn gpu_a_panel_resized_by_layout_resizes_its_surface_once_and_renders_at_the_new_size() {
    // Given
    let mut panels = Panels::new(FLEXIBLE, false);
    panels.frames(1);

    // When: the left panel narrows by 4, so the right widens by 4
    let mut doc = panels.doc.borrow_mut();
    let left = doc.children(doc.body())[0];
    doc.set_attribute(left, "class", "left narrow");
    drop(doc);
    panels.frames(2);

    // Then
    let (track, map) = (panels.track.1.borrow(), panels.map.1.borrow());
    assert_eq!(widths(&track.resizes), [12, 8]);
    assert_eq!(widths(&map.resizes), [12, 16]);
    assert_eq!(widths(&track.frames), [12, 8]);
    assert_eq!(widths(&map.frames), [12, 16]);
    assert_eq!(heights(&track.resizes), [12, 12]);
    assert_eq!(heights(&map.resizes), [12, 12]);
    assert_eq!(heights(&track.frames), [12, 12]);
    assert_eq!(heights(&map.frames), [12, 12]);
}

fn widths(sizes: &[SurfaceSize]) -> Vec<u32> {
    sizes.iter().map(|size| size.width).collect()
}

fn heights(sizes: &[SurfaceSize]) -> Vec<u32> {
    sizes.iter().map(|size| size.height).collect()
}

#[test]
fn gpu_frame_counts_count_one_composition_per_panel() {
    // Given
    let mut panels = Panels::animating(OVERSIZED);
    panels.track.0.stop_animation().expect("mounted");

    // When
    panels.frames(1);

    // Then
    assert_eq!(panels.app.frame_counts().surfaces.draw_calls, 2);

    // When both are stopped
    panels.map.0.stop_animation().expect("mounted");
    let (_, rgba) = panels.frames(1);

    // Then both stopped surfaces are still composed, from their last render
    assert_eq!(panels.app.frame_counts().surfaces.draw_calls, 2);
    assert_eq!([pixel(&rgba, 4, 4), pixel(&rgba, 20, 4)], [RED, BLUE]);
}
