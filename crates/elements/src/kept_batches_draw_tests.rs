//! Kept batches across draws of a document: a second draw at the same
//! view after an idle update makes none, and making them while an
//! earlier draw's `Ref` is held names the fix.

use vapors_scene::Rect;

/// The target: 100 x 100 device pixels.
const VIEW: Rect = Rect {
    left: 0.0,
    top: 0.0,
    right: 100.0,
    bottom: 100.0,
};

/// A document with one drawn row, updated once.
fn drawn_document() -> crate::document::Document {
    use vapors_dom::Dom;
    let mut doc = crate::document::Document::new(vapors_text::FontSystem::bundle_only());
    doc.add_stylesheet("app.css", ".row { height: 20px; background-color: red; }")
        .expect("parses");
    let (body, row) = (doc.body(), doc.create_element("div"));
    doc.set_attribute(row, "class", "row");
    doc.append(body, row);
    doc.update(UPDATE_VIEWPORT).expect("updates");
    doc
}

const UPDATE_VIEWPORT: crate::update::Viewport = crate::update::Viewport {
    width: 100.0,
    height: 100.0,
    scale: 1.0,
};

#[test]
fn a_second_draw_at_the_same_view_after_an_idle_update_makes_no_batches() {
    // Given: drawn once
    let mut doc = drawn_document();
    _ = doc.painted.as_ref().expect("painted").batches(VIEW);
    let before = crate::kept_batches::BUILDS.with(std::cell::Cell::get);

    // When: an update that changes nothing, and a draw at the same view
    let painted = doc.update(UPDATE_VIEWPORT).expect("updates");
    _ = painted.batches(VIEW);

    // Then
    assert_eq!(
        crate::kept_batches::BUILDS.with(std::cell::Cell::get),
        before
    );
}

#[test]
#[should_panic(expected = "drop the Ref before drawing")]
fn making_batches_while_a_ref_is_held_names_the_fix() {
    // Given: a draw whose Ref is held
    let doc = drawn_document();
    let painted = doc.painted.as_ref().expect("painted");
    let _held = painted.batches(VIEW);

    // When: a draw at another view
    _ = painted.batches(Rect::new(0.0, 0.0, 50.0, 50.0));
}
