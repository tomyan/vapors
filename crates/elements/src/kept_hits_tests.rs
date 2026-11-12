use vapors_dom::{Dom, NodeId};
use vapors_text::FontSystem;

use super::KeptHits;
use crate::document::Document;
use crate::update::Viewport;

const VIEWPORT: Viewport = Viewport {
    width: 200.0,
    height: 200.0,
    scale: 1.0,
};

/// Three positioned squares overlapping in a row and two blocks in
/// flow, hit tested once so their hits are kept; the squares.
fn page() -> (Document, [NodeId; 3]) {
    let mut doc = Document::new(FontSystem::bundle_only());
    let css = "body { margin: 0; } p { height: 20px; margin: 0; } \
        .sq { position: absolute; top: 40px; width: 50px; height: 50px; } \
        .a { left: 0; } .b { left: 20px; } .c { left: 40px; } \
        .wide { width: 90px; } .red { background-color: red; } .up { z-index: 1; } \
        .ghost { pointer-events: none; }";
    doc.add_stylesheet("app.css", css).expect("parses");
    let body = doc.body();
    for _ in 0..2 {
        let block = doc.create_element("p");
        doc.append(body, block);
    }
    let squares = ["sq a", "sq b", "sq c"].map(|class| {
        let square = doc.create_element("div");
        doc.set_attribute(square, "class", class);
        doc.append(body, square);
        square
    });
    doc.update(VIEWPORT).expect("updates").kept_hits();
    (doc, squares)
}

/// Asserts the kept hits after an update equal a fresh gather, naming
/// `edit`.
fn assert_kept_equals_fresh(doc: &mut Document, edit: &str) {
    let painted = doc.update(VIEWPORT).expect("updates");
    let fresh = KeptHits::of(&painted.scene());
    assert_eq!(painted.kept_hits(), &fresh, "after {edit}");
}

#[test]
fn the_kept_hits_equal_a_fresh_gather_across_a_sequence_of_edits() {
    // Given
    let (mut doc, [a, b, c]) = page();
    let edits: [(&str, NodeId, &str); 6] = [
        ("b painted red", b, "sq b red"),
        ("a made wide", a, "sq a wide"),
        ("c made wide and red", c, "sq c wide red"),
        ("b raised", b, "sq b up"),
        ("a back", a, "sq a"),
        ("b lowered", b, "sq b red"),
    ];

    for (edit, node, class) in edits {
        // When
        doc.set_attribute(node, "class", class);

        // Then
        assert_kept_equals_fresh(&mut doc, edit);
    }
    doc.remove(c);
    assert_kept_equals_fresh(&mut doc, "c removed");
}

#[test]
fn hits_patched_at_several_places_in_one_update_keep_paint_order() {
    // Given: every square hit
    let (mut doc, [a, b, c]) = page();

    // When: the first loses its hit and the last two change size in one
    // update, so the first splice shortens the list before the others
    doc.set_attribute(a, "class", "sq a ghost");
    doc.set_attribute(b, "class", "sq b wide");
    doc.set_attribute(c, "class", "sq c wide");

    // Then
    assert_kept_equals_fresh(&mut doc, "a ghosted, b and c made wide");
    let painted = doc.update(VIEWPORT).expect("updates");
    assert_eq!(
        painted.kept_hits().hit([10.0, 50.0], &painted.frames),
        None,
        "a takes no hit"
    );
    assert_eq!(
        painted.kept_hits().hit([100.0, 50.0], &painted.frames),
        Some(c)
    );

    // When: the first takes hits again and the middle one loses its own
    doc.set_attribute(a, "class", "sq a");
    doc.set_attribute(b, "class", "sq b ghost");

    // Then
    assert_kept_equals_fresh(&mut doc, "a back, b ghosted");
}

#[test]
fn the_patch_records_the_hits_removed_and_added() {
    // Given: every square hit
    let (mut doc, [a, b, c]) = page();

    // When: the first loses its hit and the last grows, in one update
    doc.set_attribute(a, "class", "sq a ghost");
    doc.set_attribute(c, "class", "sq c wide");
    let painted = doc.update(VIEWPORT).expect("updates");

    // Then: a's removed rect, c's removed and added rects, nothing else
    let changed = &painted.hits_changed;
    assert!(
        changed.may_move_hover([10.0, 50.0], None),
        "a's removed hit holds the point"
    );
    assert!(
        changed.may_move_hover([120.0, 50.0], None),
        "c's added hit holds the point"
    );
    assert!(
        changed.may_move_hover([150.0, 150.0], Some(a)),
        "a's hit was removed"
    );
    assert!(
        !changed.may_move_hover([10.0, 10.0], None),
        "the block above is untouched"
    );
    assert!(
        !changed.may_move_hover([150.0, 150.0], Some(b)),
        "b is untouched"
    );
}
