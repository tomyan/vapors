//! GPU tier: a glyph atlas full of glyphs that kept text bytes pin
//! (IU4 PR 3, `vapors_text::AtlasPins`). New text whose glyphs fit only
//! once pinned ones go clears every pin and evicts; the next frame
//! builds every text batch again, matching a painter that never kept
//! bytes, and the frame after it builds none.

#[path = "support/surface.rs"]
mod support;

use support::{Rig, bare_document, update};
use vapors_app::FrameCounts;
use vapors_dom::{Dom, NodeId};
use vapors_elements::Document;
use vapors_text::AtlasConfig;

/// Two text batches in absolutely placed boxes with backgrounds.
const CSS: &str = "div { position: absolute; left: 0; top: 0; font-size: 12px; \
    background-color: #ffffff; } .two { top: 16px; }";
/// The atlas page: holds the glyphs of `FIRST` and `SECOND`, or of
/// `FIRST` and `THIRD`, but not of all three at once.
const PAGE: u32 = 64;
const FIRST: &str = "abcdefghijklmnopqrstuvwxyz";
const SECOND: &str = "ABCDEFGHIJ";
const THIRD: &str = "KLMNOPQRSTUVWXYZ";

/// The page of `CSS` with `FIRST` and `SECOND`, updated, and the
/// second's text node.
fn page() -> (Document, NodeId) {
    let mut doc = bare_document(CSS, &[(None, "div", "one"), (None, "div", "two")]);
    let mut texts = Vec::new();
    for (at, words) in [FIRST, SECOND].into_iter().enumerate() {
        let div = doc.children(doc.body())[at];
        let text = doc.create_text(words);
        doc.append(div, text);
        texts.push(text);
    }
    update(&mut doc, 1.0);
    (doc, texts[1])
}

/// `doc` drawn by `rig`: the pixels and the frame's counts.
fn drawn(rig: &mut Rig, doc: &Document) -> (Vec<u8>, FrameCounts) {
    let pixels = rig.draw(doc).expect("paints");
    (pixels, rig.painter.frame_counts())
}

#[test]
fn gpu_new_text_in_an_atlas_of_pinned_glyphs_clears_the_pins_and_every_text_batch_is_built_again() {
    // Given: both texts drawn, their glyphs pinned
    let (mut doc, second) = page();
    let mut rig = Rig::with_atlas(AtlasConfig::fixed(PAGE));
    drawn(&mut rig, &doc);

    // When: the second text's glyphs change to ones with no room
    doc.set_text(second, THIRD);
    update(&mut doc, 1.0);
    let (in_clear, cleared) = drawn(&mut rig, &doc);
    let (pixels, rebuilt) = drawn(&mut rig, &doc);
    let (_, idle) = drawn(&mut rig, &doc);

    // Then
    assert_eq!(cleared.atlas_pin_clears, 1, "{cleared:?}");
    assert_eq!(rebuilt.atlas_pin_clears, 0, "{rebuilt:?}");
    assert_eq!(rebuilt.batches_written, 2, "both text batches: {rebuilt:?}");
    let fresh = Rig::new().draw(&doc).expect("paints");
    assert!(
        pixels == fresh,
        "text drawn from bytes kept before the clear"
    );
    assert_eq!(
        (idle.batches_written, idle.atlas_lookups),
        (0, 0),
        "{idle:?}"
    );
}
