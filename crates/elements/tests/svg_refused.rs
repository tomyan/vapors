//! What an inline `<svg>` icon refuses (docs/icons.md, The subset):
//! anything outside the subset, before layout, naming the element, the
//! attribute and the fix.

mod support;

use support::svg_page::{SVG, assert_accepted, assert_refused, icon, with_attributes};
use support::{document, element, text};

#[test]
fn the_icon_subset_is_accepted() {
    // Given
    let (mut doc, _) = icon("", &[], &[]);

    // When / Then
    assert_accepted(&mut doc);
}

#[test]
fn svg_attributes_outside_the_subset_are_refused_naming_the_fix() {
    for (changes, says) in [
        (
            &[("viewBox", "")][..],
            &["<svg class=\"icon\">", "viewBox", "required"][..],
        ),
        (&[("viewBox", "0 0 16")][..], &["viewBox", "four numbers"]),
        (
            &[("viewBox", "0 0,,16 16")][..],
            &["viewBox", "four numbers"],
        ),
        (&[("viewBox", "0 0 0 16")][..], &["viewBox", "above 0"]),
        (&[("width", "")][..], &["width", "required"]),
        (&[("width", "16px")][..], &["width", "no unit"]),
        (&[("height", "-16")][..], &["height", "positive"]),
        (
            &[("width", "32")][..],
            &["width", "height", "ratio", "viewBox"],
        ),
        (
            &[("aria-hidden", "")][..],
            &["aria-hidden=\"true\"", "role=\"img\""],
        ),
        (
            &[("aria-hidden", "false")][..],
            &["aria-hidden=\"true\"", "role=\"img\""],
        ),
        (&[("aria-hidden", ""), ("role", "img")][..], &["aria-label"]),
        (
            &[("style", "color: red")][..],
            &["style", "not in the icon subset"],
        ),
        (&[("fill", "none")][..], &["fill", "not in the icon subset"]),
        (
            &[("xmlns", "http://example.com")][..],
            &["xmlns", "is not the SVG namespace"],
        ),
    ] {
        // Given
        let (mut doc, _) = icon("", changes, &[]);

        // When / Then
        assert_refused(&mut doc, says);
    }
}

#[test]
fn the_svg_namespace_and_a_named_image_are_accepted() {
    // Given
    let named = [
        ("aria-hidden", ""),
        ("role", "img"),
        ("aria-label", "Undo"),
        ("xmlns", "http://www.w3.org/2000/svg"),
    ];
    let (mut doc, _) = icon("", &named, &[]);

    // When / Then
    assert_accepted(&mut doc);
}

#[test]
fn css_the_svg_box_does_not_follow_is_refused() {
    for (css, says) in [
        ("width: 20px;", &["width", "attributes"][..]),
        ("height: 20px;", &["height", "attributes"]),
        ("min-width: 20px;", &["min-width", "attributes"]),
        ("max-height: 20px;", &["max-height", "attributes"]),
        (
            "display: inline;",
            &["inline-block content", "display: block"],
        ),
        ("padding: 10%;", &["percentage or calc() padding"]),
        ("border-radius: 2px;", &["border-radius", "rounded corners"]),
    ] {
        // Given
        let (mut doc, _) = icon(css, &[], &[]);

        // When / Then
        assert_refused(&mut doc, says);
    }
}

#[test]
fn an_svg_holds_paths_and_whitespace_only() {
    for (child, says) in [
        ("div", &["<svg class=\"icon\">", "<div>", "path"][..]),
        ("text", &["text", "path"]),
    ] {
        // Given
        let (mut doc, svg) = icon("", &[], &[]);
        if child == "text" {
            text(&mut doc, svg, "Undo");
        } else {
            element(&mut doc, svg, child, "");
        }

        // When / Then
        assert_refused(&mut doc, says);
    }
}

#[test]
fn whitespace_between_paths_is_accepted() {
    // Given
    let (mut doc, svg) = icon("", &[], &[]);
    text(&mut doc, svg, "\n  ");

    // When / Then
    assert_accepted(&mut doc);
}

#[test]
fn an_svg_without_a_path_is_refused() {
    // Given
    let mut doc = document();
    let body = doc.body();
    let svg = element(&mut doc, body, "svg", "icon");
    with_attributes(&mut doc, svg, SVG);
    doc.add_stylesheet("app.css", ".icon { display: block; }")
        .expect("parses");

    // When / Then
    assert_refused(&mut doc, &["<svg class=\"icon\">", "no <path>"]);
}
