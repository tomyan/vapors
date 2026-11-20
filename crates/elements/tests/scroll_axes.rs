//! A scroll container on one axis (F2c S9): the axes compute together
//! (CSS Overflow 3 section 3.1), and only an axis that is `auto` or
//! `scroll` has a range, a thumb and takes the wheel; a `hidden` axis
//! clips and passes the wheel out.

use crate::support;

use support::{document, element, refusals};
use vapors_dom::NodeId;
use vapors_elements::{Document, Viewport};
use vapors_scene::{Color, Item};

const VIEW: Viewport = Viewport {
    width: 400.0,
    height: 300.0,
    scale: 1.0,
};

/// `.box` (100 x 50, `css`) holding `.wide` (300 x 160), in `.outer`
/// (200 x 100, `overflow: auto`) that holds a 500 x 400 `.huge` too.
fn page(css: &str) -> (Document, NodeId, NodeId) {
    let mut doc = document();
    let body = doc.body();
    let outer = element(&mut doc, body, "div", "outer");
    let boxed = element(&mut doc, outer, "div", "box");
    element(&mut doc, boxed, "div", "wide");
    element(&mut doc, outer, "div", "huge");
    let sheet = format!(
        "body {{ margin: 0; }} .outer {{ width: 200px; height: 100px; overflow: auto; }} \
         .box {{ width: 100px; height: 50px; scrollbar-color: red transparent; {css} }} \
         .wide {{ width: 300px; height: 160px; }} .huge {{ width: 500px; height: 400px; }}"
    );
    doc.add_stylesheet("app.css", &sheet).expect("parses");
    doc.update(VIEW).expect("updates");
    (doc, outer, boxed)
}

#[test]
fn each_axis_has_a_range_only_where_it_scrolls() {
    // Given: x then y, and the range the box reaches by scrolling to the end
    let cases = [
        ("auto", "auto", Some([200.0, 110.0])),
        ("hidden", "auto", Some([0.0, 110.0])),
        ("auto", "hidden", Some([200.0, 0.0])),
        ("visible", "auto", Some([200.0, 110.0])),
        ("auto", "visible", Some([200.0, 110.0])),
        ("clip", "scroll", Some([0.0, 110.0])),
        ("scroll", "clip", Some([200.0, 0.0])),
        ("hidden", "hidden", None),
        ("clip", "clip", None),
    ];
    for (x, y, range) in cases {
        let (mut doc, _, boxed) = page(&format!("overflow-x: {x}; overflow-y: {y};"));

        // When
        doc.scroll_to(boxed, [1000.0, 1000.0]);
        doc.update(VIEW).expect("updates");

        // Then
        assert_eq!(doc.scroll_offset(boxed), range, "x {x}, y {y}");
    }
}

#[test]
fn the_wheel_moves_only_an_axis_that_scrolls_and_the_rest_passes_out() {
    // Given: the box scrolls down only; its container both ways
    let (mut doc, outer, boxed) = page("overflow-x: hidden; overflow-y: auto;");

    // When: a sideways wheel over the box
    let moved = doc.scroll_by([10.0, 10.0], [40.0, 0.0]);

    // Then: the container took it
    assert!(moved);
    assert_eq!(doc.scroll_offset(boxed), Some([0.0, 0.0]));
    assert_eq!(doc.scroll_offset(outer), Some([40.0, 0.0]));

    // When: a wheel down
    assert!(doc.scroll_by([10.0, 10.0], [0.0, 30.0]));

    // Then: the box took it
    assert_eq!(doc.scroll_offset(boxed), Some([0.0, 30.0]));
    assert_eq!(doc.scroll_offset(outer), Some([40.0, 0.0]));
}

#[test]
fn a_sideways_box_takes_a_sideways_wheel_and_passes_a_vertical_one() {
    // Given
    let (mut doc, outer, boxed) = page("overflow-x: scroll; overflow-y: hidden;");

    // When / Then
    assert!(doc.scroll_by([10.0, 10.0], [40.0, 0.0]));
    assert_eq!(doc.scroll_offset(boxed), Some([40.0, 0.0]));
    assert!(doc.scroll_by([10.0, 10.0], [0.0, 30.0]));
    assert_eq!(doc.scroll_offset(boxed), Some([40.0, 0.0]));
    assert_eq!(doc.scroll_offset(outer), Some([0.0, 30.0]));
}

#[test]
fn content_past_a_hidden_axis_is_cut_off_there_too() {
    // Given: .wide runs to x 300 in a box 100 wide
    let (mut doc, outer, _) = page("overflow-x: hidden; overflow-y: auto;");

    // When: a point past the box's right edge, on .wide's unclipped area
    let hit = doc.update(VIEW).expect("updates").hit([150.0, 10.0]);

    // Then: the container behind it, not .wide
    assert_eq!(hit, Some(outer));
}

fn thumbs(doc: &mut Document) -> Vec<vapors_scene::Rect> {
    let red = Color {
        r: 1.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    let painted = doc.update(VIEW).expect("updates");
    let items = painted.scene().to_scene();
    items
        .items()
        .iter()
        .filter_map(|item| match item {
            Item::Quad(quad) if quad.background == red => Some(quad.rect),
            _ => None,
        })
        .collect()
}

#[test]
fn a_thumb_is_drawn_for_each_axis_that_scrolls() {
    // Given / When / Then: down only, a thumb at the right edge
    let (mut doc, _, _) = page("overflow-x: hidden; overflow-y: auto;");
    let down = thumbs(&mut doc);
    assert_eq!(down.len(), 1, "{down:?}");
    assert_eq!(down[0].right, 98.0);

    // Across only, a thumb at the bottom edge
    let (mut doc, _, _) = page("overflow-x: auto; overflow-y: hidden;");
    let across = thumbs(&mut doc);
    assert_eq!(across.len(), 1, "{across:?}");
    assert_eq!(across[0].bottom, 48.0);
}

#[test]
fn a_clip_beside_visible_is_refused_naming_the_pair_and_the_fix() {
    for (x, y) in [("clip", "visible"), ("visible", "clip")] {
        // Given
        let mut doc = document();
        let body = doc.body();
        element(&mut doc, body, "div", "box");
        let css = format!(".box {{ overflow-x: {x}; overflow-y: {y}; }}");
        doc.add_stylesheet("app.css", &css).expect("parses");

        // When
        let errors = refusals(&mut doc);

        // Then
        let said = errors.join("\n");
        assert!(
            said.contains(&format!("overflow-x: {x}; overflow-y: {y}"))
                && said.contains("give both axes clip or hidden")
                && said.contains("cargo test -p vapors-elements --test tests scroll_axes"),
            "{said}"
        );
    }
}
