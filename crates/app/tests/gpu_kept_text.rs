//! GPU tier: text and icon batches drawn from vertex bytes kept between
//! frames (IU4 PR 3, `painter_kept_text.rs`): an unchanged frame writes
//! none and looks nothing up in the atlas; text repainted in another
//! colour, whose freed text id may be issued again to the new layout, is
//! built again from the new layout.

#[path = "support/surface.rs"]
mod support;

use support::{Rig, bare_document, update};
use vapors_app::FrameCounts;
use vapors_dom::Dom;
use vapors_elements::Document;
use vapors_scene::{Item, TextId};

/// Text in two absolutely placed boxes with backgrounds, so each is its
/// own text batch; `.red` turns a box's text red.
const CSS: &str = "div { position: absolute; left: 0; font-size: 12px; \
    background-color: #ffffff; } .two { top: 14px; } .red { color: #ff0000; }";

/// The page of `CSS`, with "Ab" and "Cd", updated.
fn page() -> Document {
    let mut doc = bare_document(CSS, &[(None, "div", "one"), (None, "div", "two")]);
    for (at, words) in ["Ab", "Cd"].into_iter().enumerate() {
        let div = doc.children(doc.body())[at];
        let text = doc.create_text(words);
        doc.append(div, text);
    }
    update(&mut doc, 1.0);
    doc
}

/// The text id of `doc`'s first text item.
fn first_text(doc: &Document) -> TextId {
    let scene = doc.painted().expect("updated").scene().to_scene();
    let text = scene.items().iter().find_map(|item| match *item {
        Item::Text { text, .. } => Some(text),
        _ => None,
    });
    text.expect("a text item")
}

/// `doc` drawn by `rig`: the pixels and the frame's counts.
fn drawn(rig: &mut Rig, doc: &Document) -> (Vec<u8>, FrameCounts) {
    let pixels = rig.draw(doc).expect("paints");
    (pixels, rig.painter.frame_counts())
}

#[test]
fn gpu_an_unchanged_frame_writes_no_text_bytes_and_looks_nothing_up_in_the_atlas() {
    // Given
    let doc = page();
    let mut rig = Rig::new();
    let (first, written) = drawn(&mut rig, &doc);

    // When
    let (again, idle) = drawn(&mut rig, &doc);

    // Then: two quad batches and two text batches, all kept
    assert_eq!((written.batches_written, written.batches_kept), (4, 0));
    assert!(written.atlas_lookups > 0, "{written:?}");
    assert_eq!((idle.batches_written, idle.batches_kept), (0, 4));
    assert_eq!(idle.atlas_lookups, 0);
    assert_eq!(idle.texts.buffer_bytes, 0);
    assert_eq!(idle.retained_bytes, written.retained_bytes);
    assert!(again == first, "a frame from kept text bytes differs");
}

#[test]
fn gpu_text_repainted_in_another_colour_is_built_again_from_its_new_layout() {
    // Given
    let mut doc = page();
    let mut rig = Rig::new();
    drawn(&mut rig, &doc);
    let black = first_text(&doc);

    // When
    let div = doc.children(doc.body())[0];
    doc.set_attribute(div, "class", "one red");
    update(&mut doc, 1.0);
    let (pixels, counts) = drawn(&mut rig, &doc);

    // Then: the red layout took the black one's freed id, so the batch
    // kept its key and only the layouts tell them apart; the frame
    // matches a painter that never saw the black text
    assert_eq!(first_text(&doc), black, "the test needs the id issued again");
    let fresh = Rig::new().draw(&doc).expect("paints");
    assert!(
        pixels == fresh,
        "red text drawn from the black text's kept bytes"
    );
    assert_eq!(
        counts.texts.buffer_bytes,
        2 * 192,
        "Ab's two glyphs: {counts:?}"
    );
}
