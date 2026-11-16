//! Content overflowing a fixed-size child of a scroll container grows
//! the container's range while the child's own box does not change (4g,
//! the #431 review's M1). The child, rebuilt with the same style and
//! size, is laid out in isolation (`layout/isolated.rs`); it must answer
//! that it differs, as its scrollable overflow changed, so the container
//! is laid out again and its range measured, not kept. `Answer::is`
//! compares the whole layout output, overflow included
//! (`layout/asked.rs`); this pins it, with the placement check that
//! settles every container afresh on each debug update.

mod support;

use support::{document, element};
use vapors_dom::{Dom, NodeId};
use vapors_elements::{Document, Viewport};

const VIEW: Viewport = Viewport {
    width: 400.0,
    height: 600.0,
    scale: 1.0,
};

/// A 200 px scroller holding one 100 by 50 px item of 20 px lines.
const CSS: &str = "body { margin: 0; } .scroller { height: 200px; overflow: auto; } \
    .item { width: 100px; height: 50px; line-height: 20px; }";

/// The document, the scroller and the item's text, one line.
fn page() -> (Document, NodeId, NodeId) {
    let mut doc = document();
    doc.add_stylesheet("page.css", CSS).expect("parses");
    let body = doc.body();
    let scroller = element(&mut doc, body, "div", "scroller");
    let item = element(&mut doc, scroller, "div", "item");
    let text = doc.create_text("a");
    doc.append(item, text);
    doc.update(VIEW).expect("updates");
    (doc, scroller, text)
}

#[test]
fn text_overflowing_a_fixed_size_child_grows_the_range() {
    // Given: nothing to scroll
    let (mut doc, scroller, text) = page();
    doc.scroll_to(scroller, [0.0, 1.0e6]);
    doc.update(VIEW).expect("updates");
    assert_eq!(doc.scroll_offset(scroller), Some([0.0, 0.0]));

    // When: the text wraps past the item's 50 px, its box unchanged
    doc.set_text(text, &"word ".repeat(80));
    doc.update(VIEW).expect("updates");
    let measured = doc.update_counts().scroll_ranges_measured;
    doc.scroll_to(scroller, [0.0, 1.0e6]);
    doc.update(VIEW).expect("updates");

    // Then: the range was measured, and reaches the overflow's end
    assert_eq!(measured, 1);
    let end = doc.scroll_offset(scroller).expect("scrolls")[1];
    assert!(end > 100.0, "{end}");
}
