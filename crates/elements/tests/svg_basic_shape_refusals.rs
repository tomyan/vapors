//! What a `<rect>` or `<circle>` refuses at update time (one-scene
//! S4b), for what the build cannot see: a value set at run time, a
//! shape outside an svg, a shape holding content. Each is refused
//! naming the attribute or the fix. A change to one alone rechecks its
//! svg.

use crate::support;

use support::svg_page::{assert_refused, changed};
use support::svg_shapes_page::{Attributes, CIRCLE, RECT, drawn, page};
use support::{document, element};
use vapors_dom::Dom;

#[test]
fn a_rect_or_circle_refuses_what_svg_would_read_otherwise() {
    // Given: a unit; a percentage; a transform; a missing radius
    let cases: [(&str, Attributes, &[&str]); 4] = [
        (
            "rect",
            &[("width", "12px")],
            &["<rect>", "width \"12px\" is not a bare number"],
        ),
        (
            "rect",
            &[("x", "10%")],
            &["<rect>", "x \"10%\" is not a bare number"],
        ),
        (
            "rect",
            &[("transform", "scale(2)")],
            &["`transform` is not an attribute of <rect>"],
        ),
        (
            "circle",
            &[("r", "")],
            &["<circle>", "its r attribute is required"],
        ),
    ];

    for (tag, change, says) in cases {
        let base = if tag == "rect" { RECT } else { CIRCLE };
        let (mut doc, _) = page(&[&[(tag, &changed(base, change))]]);

        // When, Then
        assert_refused(&mut doc, says);
    }
}

#[test]
fn a_rect_changed_alone_to_reach_outside_the_view_box_is_refused() {
    // Given: updated once
    let (mut doc, svgs) = page(&[&[("rect", RECT)]]);
    drawn(&mut doc);
    let rect = doc.children(svgs[0])[0];

    // When: only the rect changes
    doc.set_attribute(rect, "width", "20");

    // Then: refused, naming it
    assert_refused(&mut doc, &["its rect 1 draws from", "outside its viewBox"]);
}

#[test]
fn a_rect_outside_an_svg_is_refused() {
    // Given: a rect straight in the body
    let mut doc = document();
    let body = doc.body();
    let rect = element(&mut doc, body, "rect", "");
    for (name, value) in RECT {
        doc.set_attribute(rect, name, value);
    }

    // When, Then
    assert_refused(&mut doc, &["<rect>", "is not inside an <svg>"]);
}

#[test]
fn a_circle_holding_content_is_refused() {
    // Given
    let (mut doc, svgs) = page(&[&[("circle", CIRCLE)]]);
    let circle = doc.children(svgs[0])[0];
    let text = doc.create_text("dot");
    doc.append(circle, text);

    // When, Then
    assert_refused(
        &mut doc,
        &["<circle>", "holds content, and a circle holds none"],
    );
}
