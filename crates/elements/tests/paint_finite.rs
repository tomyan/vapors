//! Paint refuses a box whose border box has an edge that is not finite
//! in device pixels, naming the element and its slot, once each, rather
//! than put an infinite quad in the scene (issue #449): a rect is
//! placed, or the box is refused; a box with no rect is not painted.

use crate::support;

use support::{document, element, viewport};
use vapors_elements::Viewport;

const NOT_FINITE: &str = "has an edge that is not finite in device pixels";

/// A page of one `div.wide` styled by `css`, updated at `at`: the
/// refusals that say an edge is not finite.
fn refusals_at(css: &str, at: Viewport) -> Vec<String> {
    let mut doc = document();
    let body = doc.body();
    element(&mut doc, body, "div", "wide");
    doc.add_stylesheet("wide.css", css).expect("parses");
    match doc.update(at) {
        Ok(_) => Vec::new(),
        Err(errors) => (errors.iter().map(ToString::to_string))
            .filter(|error| error.contains(NOT_FINITE))
            .collect(),
    }
}

#[test]
fn a_box_past_f32_s_range_is_refused_once_naming_the_element_and_slot() {
    // When: layout keeps a width past f32's range as infinity; the
    // box paints a quad, which the debug whole-pixel check would meet
    let css = ".wide { width: 1e39px; height: 10px; background-color: red; }";
    let refused = refusals_at(css, viewport(640.0, 400.0));

    // Then
    assert_eq!(refused.len(), 1, "{refused:?}");
    assert!(
        refused[0].starts_with("<div class=\"wide\"> (box slot "),
        "{}",
        refused[0]
    );
    assert!(refused[0].contains("width inf"), "{}", refused[0]);
}

#[test]
fn a_box_finite_in_css_pixels_but_not_on_the_device_is_refused() {
    // Given: 2e38 px is within f32's range; at scale 2 its right edge is not
    let css = ".wide { width: 2e38px; }";
    let scaled = Viewport {
        scale: 2.0,
        ..viewport(640.0, 400.0)
    };

    // When
    let at_one = refusals_at(css, viewport(640.0, 400.0));
    let at_two = refusals_at(css, scaled);

    // Then
    assert_eq!(at_one, Vec::<String>::new());
    assert_eq!(at_two.len(), 1, "{at_two:?}");
    assert!(at_two[0].contains("at scale 2"), "{}", at_two[0]);
}
