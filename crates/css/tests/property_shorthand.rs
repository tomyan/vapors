//! Box shorthands (`src/property/shorthand.rs`).

mod support;

use support::property_shorthand::longhands;
use support::{refusal, sheet};

#[test]
fn margin_and_padding_expand_one_to_four_values() {
    assert_eq!(
        longhands("margin: 1px"),
        [
            "margin-top: 1px",
            "margin-right: 1px",
            "margin-bottom: 1px",
            "margin-left: 1px"
        ]
    );
    assert_eq!(
        longhands("padding: 1px 2px"),
        [
            "padding-top: 1px",
            "padding-right: 2px",
            "padding-bottom: 1px",
            "padding-left: 2px"
        ]
    );
    assert_eq!(
        longhands("margin: 1px 2px 3px"),
        [
            "margin-top: 1px",
            "margin-right: 2px",
            "margin-bottom: 3px",
            "margin-left: 2px"
        ]
    );
    assert_eq!(
        longhands("inset: 1px 2px 3px 4px"),
        ["top: 1px", "right: 2px", "bottom: 3px", "left: 4px"]
    );
    assert!(
        refusal("a { margin: 1px 2px 3px 4px 5px; }")
            .message
            .contains("margin")
    );
}

#[test]
fn border_sides_expand() {
    assert_eq!(
        longhands("border-width: 1px 2px"),
        [
            "border-top-width: 1px",
            "border-right-width: 2px",
            "border-bottom-width: 1px",
            "border-left-width: 2px"
        ]
    );
    assert_eq!(
        longhands("border-color: red")[2],
        "border-bottom-color: rgb(255 0 0)"
    );
    assert_eq!(
        longhands("border-style: solid none")[1],
        "border-right-style: none"
    );
}

#[test]
fn border_radius_expands_corners() {
    assert_eq!(
        longhands("border-radius: 4px"),
        [
            "border-top-left-radius: 4px",
            "border-top-right-radius: 4px",
            "border-bottom-right-radius: 4px",
            "border-bottom-left-radius: 4px"
        ]
    );
    assert_eq!(
        longhands("border-radius: 1px 2px")[3],
        "border-bottom-left-radius: 2px"
    );
    assert!(
        refusal("a { border-radius: 1px / 2px; }")
            .message
            .contains('/')
    );
}

#[test]
fn gap_overflow_and_flex_expand() {
    assert_eq!(longhands("gap: 4px"), ["row-gap: 4px", "column-gap: 4px"]);
    assert_eq!(
        longhands("gap: 4px 8px"),
        ["row-gap: 4px", "column-gap: 8px"]
    );
    assert_eq!(
        longhands("overflow: hidden"),
        ["overflow-x: hidden", "overflow-y: hidden"]
    );
    assert_eq!(
        longhands("overflow: hidden auto"),
        ["overflow-x: hidden", "overflow-y: auto"]
    );
    assert_eq!(
        longhands("flex: 1"),
        ["flex-grow: 1", "flex-shrink: 1", "flex-basis: 0px"]
    );
    assert_eq!(
        longhands("flex: none"),
        ["flex-grow: 0", "flex-shrink: 0", "flex-basis: auto"]
    );
    assert_eq!(
        longhands("flex: auto"),
        ["flex-grow: 1", "flex-shrink: 1", "flex-basis: auto"]
    );
    assert_eq!(
        longhands("flex: 2 0 10px"),
        ["flex-grow: 2", "flex-shrink: 0", "flex-basis: 10px"]
    );
    assert_eq!(
        longhands("flex: 0 0"),
        ["flex-grow: 0", "flex-shrink: 0", "flex-basis: 0px"]
    );
}

#[test]
fn important_applies_to_every_longhand() {
    let s = sheet("a { margin: 1px !important; }");
    assert!(s.rules[0].declarations[0].important);
    assert_eq!(s.rules[0].declarations[0].longhands.len(), 4);
}
