//! The `border` shorthand (`src/property/shorthand_border.rs`).

mod support;

use support::property_shorthand::longhands;

#[test]
fn border_shorthand_sets_width_style_and_colour_in_any_order() {
    // When
    let got = longhands("border: red 1px solid");

    // Then
    assert_eq!(got.len(), 12);
    assert!(got.contains(&"border-top-width: 1px".to_owned()));
    assert!(got.contains(&"border-left-style: solid".to_owned()));
    assert!(got.contains(&"border-bottom-color: rgb(255 0 0)".to_owned()));
}

#[test]
fn border_shorthand_fills_omitted_parts_with_initial_values() {
    let got = longhands("border: 2px");
    assert!(got.contains(&"border-top-style: none".to_owned()));
    assert!(got.contains(&"border-top-color: currentcolor".to_owned()));
    assert_eq!(longhands("border: none").len(), 12);
}

#[test]
fn a_side_shorthand_sets_only_that_sides_width_style_and_colour() {
    // When
    let got = longhands("border-top: thin solid red");

    // Then
    assert_eq!(
        got,
        [
            "border-top-width: thin",
            "border-top-style: solid",
            "border-top-color: rgb(255 0 0)"
        ]
    );
}

#[test]
fn a_side_shorthand_fills_omitted_parts_and_refuses_a_fourth() {
    // When
    let filled = longhands("border-left: 2px");
    let refused = vapors_css::parse("a { border-left: 2px solid red blue; }");

    // Then
    assert!(
        filled.contains(&"border-left-style: none".to_owned()),
        "{filled:?}"
    );
    let error = refused.expect_err("a fourth part is refused");
    assert!(error.message.contains("border-left"), "{}", error.message);
}
