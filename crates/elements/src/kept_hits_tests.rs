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
        .wide { width: 90px; } .red { background-color: red; } .up { z-index: 1; }";
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
