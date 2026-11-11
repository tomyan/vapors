//! Parts kept in place (IU3 3c-3d2, `docs/plans/iu3.md` "The scene in
//! stable slots"): the scene is a view over paint order and each slot's
//! part, hit testing walks that order backwards, and a removed
//! element's part leaves no entry behind. A debug build also compares
//! every update's paint, hits and maps with a full paint's
//! (`kept_paint_check.rs`).

mod support;

use support::rows_page::{EDITS, rows_page, rows_viewport};
use support::{document, element, viewport};
use vapors_dom::{Dom, NodeId};
use vapors_elements::{Cursor, Document};
use vapors_scene::Rect;

/// Two 50px squares at the top left, the second 20px right of the
/// first; `under` with z-index 1, `over` with z-index 2; hit tested
/// once, so a later update's hits are gathered again.
fn overlapping() -> (Document, NodeId, NodeId) {
    let mut doc = document();
    let css = "body { margin: 0; } \
        div { position: absolute; top: 0; width: 50px; height: 50px; } \
        .over { left: 0; z-index: 2; } .under { left: 20px; z-index: 1; } \
        .red { background-color: red; } .top { z-index: 3; }";
    doc.add_stylesheet("app.css", css).expect("parses");
    let body = doc.body();
    let over = element(&mut doc, body, "div", "over");
    let under = element(&mut doc, body, "div", "under");
    let painted = doc.update(viewport(200.0, 200.0)).expect("updates");
    assert_eq!(painted.hit([30.0, 10.0]), Some(over));
    (doc, over, under)
}

#[test]
fn a_repainted_box_keeps_its_place_in_hit_order() {
    // Given
    let (mut doc, over, under) = overlapping();

    // When: the lower box is built and painted again, last
    doc.set_attribute(under, "class", "under red");
    let painted = doc.update(viewport(200.0, 200.0)).expect("updates");

    // Then: where both are, the upper one still takes the pointer
    assert_eq!(painted.hit([30.0, 10.0]), Some(over));
    assert_eq!(painted.hit([60.0, 10.0]), Some(under));
}

#[test]
fn a_box_raised_over_another_takes_the_hit() {
    // Given
    let (mut doc, _, under) = overlapping();

    // When
    doc.set_attribute(under, "class", "under top");
    let painted = doc.update(viewport(200.0, 200.0)).expect("updates");

    // Then
    assert_eq!(painted.hit([30.0, 10.0]), Some(under));
}

#[test]
fn a_removed_element_has_no_border_box_cursor_hit_or_visible_entry() {
    // Given: a 50px square with a pointer cursor, alone in the body
    let mut doc = document();
    doc.add_stylesheet(
        "app.css",
        "body { margin: 0; } div { width: 50px; height: 50px; cursor: pointer; }",
    )
    .expect("parses");
    let body = doc.body();
    let square = element(&mut doc, body, "div", "");
    let shown = doc.update(viewport(200.0, 200.0)).expect("updates");
    assert_eq!(shown.cursor([10.0, 10.0]), Cursor::Pointer);

    // When
    doc.remove(square);
    let painted = doc.update(viewport(200.0, 200.0)).expect("updates");

    // Then
    assert_eq!(painted.border_box(square), None);
    assert!(!painted.is_visible(square));
    assert_ne!(painted.hit([10.0, 10.0]), Some(square));
    assert_eq!(painted.cursor([10.0, 10.0]), Cursor::Default);
}

#[test]
fn batching_the_view_equals_batching_its_flat_scene_on_the_rows_page() {
    // Given: 100 rows after every edit, some text clipped by its value box
    let page = rows_page(100);
    for edit in EDITS {
        page.edit(edit);
        page.doc
            .borrow_mut()
            .update(rows_viewport())
            .expect("updates");
    }
    let doc = page.doc.borrow();
    let scene = doc.painted().expect("painted").scene();
    let view = Rect::new(0.0, 0.0, 640.0, 400.0);

    // When
    let batched = scene.batches(view);

    // Then
    assert_eq!(batched, scene.to_scene().batches(view));
    assert!(batched.culled > 0, "the rows below the view are culled");
    assert!(batched.batches.iter().any(|batch| batch.clip.is_some()));
}
