//! Which scroll container a scroll at a point moves (F2c S9): the
//! innermost under the point that can still move that way, else the
//! next one out; and which boxes move with it, by their containing
//! blocks, as S8's clips follow them.

mod support;

use support::{document, element};
use vapors_dom::NodeId;
use vapors_elements::{Document, Viewport};

const VIEWPORT: Viewport = Viewport {
    width: 300.0,
    height: 300.0,
    scale: 1.0,
};

fn top(doc: &mut Document, node: NodeId) -> f32 {
    doc.update(VIEWPORT)
        .expect("updates")
        .border_box(node)
        .expect("a box")
        .y
}

/// `.outer` (100 x 100, scrolls, 200 px of content: a 60 px `.inner`
/// that scrolls 40 px of its 100, then a 140 px `.tail`).
fn nested() -> (Document, NodeId, NodeId) {
    let mut doc = document();
    let body = doc.body();
    let outer = element(&mut doc, body, "div", "outer");
    let inner = element(&mut doc, outer, "div", "inner");
    element(&mut doc, inner, "div", "filler");
    element(&mut doc, outer, "div", "tail");
    doc.add_stylesheet(
        "app.css",
        "body { margin: 0; } .outer { height: 100px; overflow: auto; } \
         .inner { height: 60px; overflow: auto; } .filler { height: 100px; } .tail { height: 140px; }",
    )
    .expect("parses");
    doc.update(VIEWPORT).expect("updates");
    (doc, outer, inner)
}

#[test]
fn the_innermost_container_under_the_point_scrolls_first() {
    // Given
    let (mut doc, outer, inner) = nested();

    // When
    doc.scroll_by([10.0, 10.0], [0.0, 25.0]);

    // Then
    assert_eq!(doc.scroll_offset(inner), Some([0.0, 25.0]));
    assert_eq!(doc.scroll_offset(outer), Some([0.0, 0.0]));
}

#[test]
fn a_container_at_its_end_passes_the_scroll_out() {
    // Given: the inner one at its end
    let (mut doc, outer, inner) = nested();
    doc.scroll_by([10.0, 10.0], [0.0, 40.0]);
    doc.update(VIEWPORT).expect("updates");

    // When
    let scrolled = doc.scroll_by([10.0, 10.0], [0.0, 25.0]);

    // Then
    assert!(scrolled);
    assert_eq!(doc.scroll_offset(inner), Some([0.0, 40.0]));
    assert_eq!(doc.scroll_offset(outer), Some([0.0, 25.0]));
}

#[test]
fn a_scroll_no_container_can_take_does_nothing() {
    // Given
    let (mut doc, outer, inner) = nested();

    // When: up, with both at their start
    let scrolled = doc.scroll_by([10.0, 10.0], [0.0, -25.0]);

    // Then
    assert!(!scrolled);
    assert_eq!(doc.scroll_offset(inner), Some([0.0, 0.0]));
    assert_eq!(doc.scroll_offset(outer), Some([0.0, 0.0]));
}

/// A scrolling `.list` holding `.moved` (in flow), and `child_css`'s
/// `.child`; `.list` has `list_css`.
fn with_child(list_css: &str, child_css: &str) -> (Document, NodeId) {
    let mut doc = document();
    let body = doc.body();
    let list = element(&mut doc, body, "div", "list");
    element(&mut doc, list, "div", "filler");
    let child = element(&mut doc, list, "div", "child");
    let css = format!(
        "body {{ margin: 0; }} .list {{ height: 50px; overflow: auto; {list_css} }} .filler {{ height: 200px; }} \
         .child {{ width: 10px; height: 10px; top: 5px; {child_css} }}"
    );
    doc.add_stylesheet("app.css", &css).expect("parses");
    doc.update(VIEWPORT).expect("updates");
    doc.scroll_by([10.0, 10.0], [0.0, 30.0]);
    (doc, child)
}

#[test]
fn an_absolute_box_placed_by_the_container_scrolls_with_it() {
    // Given
    let (mut doc, child) = with_child("position: relative;", "position: absolute;");

    // When / Then
    assert_eq!(top(&mut doc, child), 5.0 - 30.0);
}

#[test]
fn an_absolute_box_placed_by_a_box_outside_the_container_does_not_scroll() {
    // Given
    let (mut doc, child) = with_child("", "position: absolute;");

    // When / Then
    assert_eq!(top(&mut doc, child), 5.0);
}

#[test]
fn a_fixed_box_does_not_scroll() {
    // Given
    let (mut doc, child) = with_child("position: relative;", "position: fixed;");

    // When / Then
    assert_eq!(top(&mut doc, child), 5.0);
}
