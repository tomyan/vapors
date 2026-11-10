//! What a `path` of an inline `<svg>` icon refuses (docs/icons.md, The
//! subset), before layout, naming the element, the attribute and the
//! fix.

mod support;

use support::svg_page::{PATH, assert_accepted, assert_refused, icon, with_attributes};
use support::{document, element};

#[test]
fn path_attributes_outside_the_subset_are_refused_naming_the_fix() {
    for (changes, says) in [
        (&[("d", "")][..], &["<path>", "d", "required"][..]),
        (&[("d", "M2 8C1 1 2 2 3 3")][..], &["<path>", "byte 4"]),
        (&[("fill", "")][..], &["fill=\"none\""]),
        (&[("fill", "red")][..], &["fill=\"none\""]),
        (&[("stroke", "")][..], &["stroke=\"currentColor\""]),
        (&[("stroke", "#fff")][..], &["stroke=\"currentColor\""]),
        (&[("stroke-width", "0")][..], &["stroke-width", "positive"]),
        (&[("stroke-width", "1px")][..], &["stroke-width", "no unit"]),
        (
            &[("transform", "scale(2)")][..],
            &["transform", "not in the icon subset"],
        ),
        (
            &[("class", "line")][..],
            &["class", "not in the icon subset"],
        ),
    ] {
        // Given
        let (mut doc, _) = icon("", &[], changes);

        // When / Then
        assert_refused(&mut doc, says);
    }
}

#[test]
fn a_path_without_a_stroke_width_takes_one_as_in_svg() {
    // Given
    let (mut doc, _) = icon("", &[], &[("stroke-width", "")]);

    // When / Then
    assert_accepted(&mut doc);
}

#[test]
fn a_path_outside_an_svg_is_refused() {
    // Given
    let mut doc = document();
    let body = doc.body();
    let path = element(&mut doc, body, "path", "");
    with_attributes(&mut doc, path, PATH);

    // When / Then
    assert_refused(&mut doc, &["<path>", "inside an <svg>"]);
}

#[test]
fn a_path_styled_apart_from_its_svg_is_refused() {
    for (css, says) in [
        ("color: red;", &["<path>", "color"][..]),
        ("display: none;", &["<path>", "display"]),
        ("visibility: hidden;", &["<path>", "visibility"]),
    ] {
        // Given
        let (mut doc, _) = icon("", &[], &[]);
        doc.add_stylesheet("path.css", &format!("path {{ {css} }}"))
            .expect("parses");

        // When / Then
        assert_refused(&mut doc, says);
    }
}
