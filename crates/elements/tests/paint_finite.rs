//! Paint refuses an update in which a box would draw a coordinate that
//! is not finite in device pixels (issue #449): its border box, any item
//! it paints (a shadow, an outline ring, an image, text, a shape) or a
//! clip. One refusal names the cause, the first box in tree order whose
//! own length is past f32's range, or else the first box drawn, with the
//! property; the boxes it reaches are counted, not each refused.

use crate::support;

use support::{document, element, viewport};
use vapors_dom::Dom;
use vapors_elements::{Document, Viewport};

const NOT_FINITE: &str = "not finite in device pixels";

/// A page of one `div.a` and, after it, one `div.b`, styled by `css`.
fn page(css: &str) -> Document {
    let mut doc = document();
    let body = doc.body();
    element(&mut doc, body, "div", "a");
    element(&mut doc, body, "div", "b");
    doc.add_stylesheet("t.css", css).expect("parses");
    doc
}

/// The one refusal of `css`'s page updated at `at`.
fn refusal(css: &str, at: Viewport) -> String {
    let mut doc = page(css);
    let errors = doc.update(at).err().expect("refused");
    let messages: Vec<String> = errors.iter().map(ToString::to_string).collect();
    assert_eq!(messages.len(), 1, "{messages:?}");
    assert!(messages[0].contains(NOT_FINITE), "{}", messages[0]);
    messages.into_iter().next().expect("one")
}

const BOX: &str = ".a { width: 10px; height: 10px; } ";

#[test]
fn a_width_past_f32_s_range_is_refused_naming_the_element_and_property() {
    // When
    let refused = refusal(
        ".a { width: 1e39px; height: 10px; background-color: red; }",
        viewport(640.0, 400.0),
    );

    // Then
    assert!(
        refused.starts_with("<div class=\"a\"> (box slot "),
        "{refused}"
    );
    assert!(refused.contains("`width`"), "{refused}");
    assert!(refused.contains("reproduce: "), "{refused}");
}

#[test]
fn a_box_shadow_past_f32_s_range_is_refused_naming_box_shadow() {
    for shadow in ["1e39px 0 red", "0 0 0 1e39px red", "0 0 1e39px red"] {
        // When: the border box is finite; the shadow is not
        let refused = refusal(
            &format!("{BOX}.a {{ box-shadow: {shadow}; }}"),
            viewport(640.0, 400.0),
        );

        // Then
        assert!(
            refused.starts_with("<div class=\"a\">"),
            "{shadow}: {refused}"
        );
        assert!(
            refused.contains("its shadow (`box-shadow`)"),
            "{shadow}: {refused}"
        );
    }
}

#[test]
fn a_shadow_finite_in_css_pixels_but_not_on_the_device_is_refused() {
    // Given
    let css = format!("{BOX}.a {{ box-shadow: 2e38px 0 red; }}");
    let scaled = Viewport {
        scale: 2.0,
        ..viewport(640.0, 400.0)
    };

    // When
    let at_one = page(&css)
        .update(viewport(640.0, 400.0))
        .map(|_| ())
        .map_err(|e| format!("{e:?}"));
    let at_two = refusal(&css, scaled);

    // Then
    assert_eq!(at_one, Ok(()));
    assert!(at_two.contains("at scale 2"), "{at_two}");
}

#[test]
fn an_outline_past_f32_s_range_is_refused_naming_outline() {
    for outline in [
        "outline: 1px solid red; outline-offset: 1e39px;",
        "outline: 1e39px solid red;",
    ] {
        // When: the border box is finite; the ring is not
        let refused = refusal(&format!("{BOX}.a {{ {outline} }}"), viewport(640.0, 400.0));

        // Then
        assert!(
            refused.starts_with("<div class=\"a\">"),
            "{outline}: {refused}"
        );
        assert!(
            refused.contains("its outline ring (`outline-width` or `outline-offset`)"),
            "{outline}: {refused}"
        );
    }
}

#[test]
fn a_margin_that_pushes_others_past_f32_s_range_is_refused_once_naming_its_owner() {
    // When: .a is finite, its margin places .b, body and html at infinity
    let refused = refusal(
        ".a { height: 10px; margin-bottom: 1e39px; } .b { height: 10px; background-color: red; }",
        viewport(640.0, 400.0),
    );

    // Then
    assert!(refused.starts_with("<div class=\"a\">"), "{refused}");
    assert!(refused.contains("`margin-bottom`"), "{refused}");
    assert!(refused.contains("other boxes"), "{refused}");
}

#[test]
fn after_a_refusal_nothing_is_shown_and_a_fixed_style_updates() {
    // Given: refused
    let mut doc = page(".a { width: 1e39px; height: 10px; } .ok { width: 10px; height: 10px; }");
    assert!(doc.update(viewport(640.0, 400.0)).is_err());
    assert!(doc.painted().is_none());
    let a = doc.children(doc.body())[0];

    // When: the element no longer takes the rule
    doc.set_attribute(a, "class", "ok");
    let updated = doc
        .update(viewport(640.0, 400.0))
        .map(|_| ())
        .map_err(|e| format!("{e:?}"));

    // Then
    assert_eq!(updated, Ok(()));
    assert!(doc.painted().is_some());
}
