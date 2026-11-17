//! GPU tier: quads under different clips in one frame join one batch
//! (flat-scene F3.1, step 2, `docs/plans/flat-scene-f3.md` item 10). A
//! strip of boxes in the page's frame, each clipping a wider one: each
//! quad is under its own box's clip (`overflow: hidden` clips without
//! making a frame). Before step 2 a clip change split the run, a draw a
//! box; joined, they are one draw, each quad naming its clip's entry in
//! the clip table.

use crate::support_surface as support;

use support::{Rig, document, update};
use vapors_dom::Dom;

/// A strip of four 3 px boxes, each clipping a 6 px wide red box: 15
/// CSS px, inside the rig's 16 (32 device px at 2x).
const CSS: &str = "body { margin: 0; } .strip { display: flex; gap: 1px; } \
    .cut { overflow: hidden; width: 3px; height: 4px; } \
    .wide { width: 6px; height: 4px; background-color: #f00; } \
    .short { height: 2px; }";

/// The boxes in the strip.
const BOXES: usize = 4;

/// The strip of `BOXES` boxes, each clipping a wider red one, updated.
fn strip() -> vapors_elements::Document {
    let mut elements = vec![(None, "div", "strip")];
    for at in 0..BOXES {
        elements.push((Some(0), "div", "cut"));
        elements.push((Some(1 + at * 2), "div", "wide"));
    }
    let mut doc = document(CSS, &elements);
    update(&mut doc, 2.0);
    doc
}

#[test]
fn gpu_quads_each_under_its_own_clip_in_one_frame_are_one_draw() {
    // Given: four boxes, each quad under its own box's clip
    let doc = strip();

    // When
    let mut rig = Rig::new();
    let pixels = rig.draw(&doc).expect("paints");

    // Then: one quad draw for the four quads under four clips
    let counts = rig.painter.frame_counts();
    assert_eq!(counts.quads.draw_calls, 1, "{counts:?}");
    assert_eq!(counts.clip_entries_changed, BOXES as u64, "{counts:?}");
    // and each quad cut by its own box: 3 by 4 CSS px of red a box, at
    // 2 device px a CSS px
    let red = pixels
        .chunks(4)
        .filter(|px| px[0] > 200 && px[1] < 50 && px[2] < 50)
        .count();
    assert_eq!(red, BOXES * 3 * 4 * 4);
}

#[test]
fn gpu_a_joined_batch_whose_one_clip_changed_is_built_again() {
    // Given: the strip drawn, its quads kept as one batch
    let mut doc = strip();
    let mut rig = Rig::new();
    rig.draw(&doc).expect("paints");

    // When: the second box grows shorter, so its clip changes, while its
    // quad, and every other box, do not move: the batch's items, and so
    // its key, stay as they were
    let second = doc.children(doc.children(doc.body())[0])[1];
    doc.set_attribute(second, "class", "cut short");
    update(&mut doc, 2.0);
    let pixels = rig.draw(&doc).expect("paints");

    // Then: built again, naming the new entry, and drawn as afresh
    let counts = rig.painter.frame_counts();
    assert_eq!(counts.batches_written, 1, "{counts:?}");
    assert!(
        pixels == Rig::new().draw(&doc).expect("paints"),
        "drawn other than afresh"
    );
}
