//! Layout reads the computed pair (CSS Overflow 3 section 3.1): a flex
//! item with one scrolling axis is a scroll container on both, so its
//! automatic minimum size is 0 and its range is per axis.

use crate::support;

use support::{document, element};
use vapors_dom::NodeId;
use vapors_elements::{Document, Viewport};

const VIEW: Viewport = Viewport {
    width: 400.0,
    height: 300.0,
    scale: 1.0,
};

/// `.flex` (`direction`, 100 x 100) holding `.item` (`css`) with a
/// 300 x 300 child.
fn flex(direction: &str, css: &str) -> (Document, NodeId) {
    let mut doc = document();
    let body = doc.body();
    let flex = element(&mut doc, body, "div", "flex");
    let item = element(&mut doc, flex, "div", "item");
    element(&mut doc, item, "div", "inner");
    let sheet = format!(
        "body {{ margin: 0; }} .flex {{ display: flex; flex-direction: {direction}; width: 100px; height: 100px; }} \
         .item {{ {css} }} .inner {{ width: 300px; height: 300px; }}"
    );
    doc.add_stylesheet("app.css", &sheet).expect("parses");
    (doc, item)
}

fn size(doc: &mut Document, item: NodeId) -> (f32, f32) {
    let b = doc
        .update(VIEW)
        .expect("updates")
        .border_box(item)
        .expect("a box");
    (b.width, b.height)
}

#[test]
fn one_scrolling_axis_lets_a_flex_item_shrink_on_both() {
    for (direction, css) in [
        ("column", "overflow-y: auto;"),
        ("column", "overflow-x: hidden;"),
        ("row", "overflow-y: auto;"),
        ("row", "overflow-x: scroll;"),
    ] {
        // Given
        let (mut doc, item) = flex(direction, css);

        // When / Then: the item fits the container along the main axis
        let (w, h) = size(&mut doc, item);
        let main = if direction == "row" { w } else { h };
        assert_eq!(main, 100.0, "{direction} {css}");
    }
}

#[test]
fn a_visible_pair_does_not_shrink_below_its_content() {
    // Given
    let (mut doc, item) = flex("column", "");

    // When / Then
    assert_eq!(size(&mut doc, item).1, 300.0);
}

#[test]
fn the_range_of_a_shrunk_item_is_its_content_past_its_box_on_each_axis() {
    // Given: 300 x 300 in a 100-tall column, wide as the item (100: stretch)
    let (mut doc, item) = flex("column", "overflow-y: auto; overflow-x: hidden;");
    doc.update(VIEW).expect("updates");

    // When
    doc.scroll_to(item, [500.0, 500.0]);

    // Then
    assert_eq!(doc.scroll_offset(item), Some([0.0, 200.0]));
}
