//! `overflow-x` and `overflow-y` compute together (CSS Overflow 3
//! section 3.1): `visible` becomes `auto` and `clip` becomes `hidden`
//! on one axis when the other is neither `visible` nor `clip`.

use crate::support;

use support::css_at;

const KEYWORDS: [&str; 5] = ["visible", "hidden", "clip", "auto", "scroll"];

/// The table of the specification, by row (x) and column (y) in
/// `KEYWORDS` order, each cell `x/y`; written out, not derived.
const TABLE: [[&str; 5]; 5] = [
    [
        "visible/visible",
        "auto/hidden",
        "visible/clip",
        "auto/auto",
        "auto/scroll",
    ],
    [
        "hidden/auto",
        "hidden/hidden",
        "hidden/hidden",
        "hidden/auto",
        "hidden/scroll",
    ],
    [
        "clip/visible",
        "hidden/hidden",
        "clip/clip",
        "hidden/auto",
        "hidden/scroll",
    ],
    [
        "auto/auto",
        "auto/hidden",
        "auto/hidden",
        "auto/auto",
        "auto/scroll",
    ],
    [
        "scroll/auto",
        "scroll/hidden",
        "scroll/hidden",
        "scroll/auto",
        "scroll/scroll",
    ],
];

#[test]
fn every_pair_computes_as_the_specification_says() {
    for (xi, x) in KEYWORDS.iter().enumerate() {
        for (yi, y) in KEYWORDS.iter().enumerate() {
            // Given
            let css = format!(".b {{ overflow-x: {x}; overflow-y: {y}; }}");

            // When
            let got = format!(
                "{}/{}",
                css_at(&css, &["div.b"], "overflow-x"),
                css_at(&css, &["div.b"], "overflow-y")
            );

            // Then
            assert_eq!(got, TABLE[xi][yi], "overflow-x: {x}; overflow-y: {y}");
        }
    }
}

#[test]
fn an_undeclared_axis_is_visible_and_computes_with_the_other() {
    // When / Then
    let css = ".b { overflow-y: auto; }";
    assert_eq!(css_at(css, &["div.b"], "overflow-x"), "auto");
    let css = ".b { overflow-x: hidden; }";
    assert_eq!(css_at(css, &["div.b"], "overflow-y"), "auto");
}

#[test]
fn the_shorthand_gives_both_axes() {
    // When / Then
    let css = ".b { overflow: hidden auto; }";
    assert_eq!(css_at(css, &["div.b"], "overflow-x"), "hidden");
    assert_eq!(css_at(css, &["div.b"], "overflow-y"), "auto");
}

#[test]
fn overflow_is_not_inherited() {
    // When / Then
    let css = ".p { overflow-y: auto; }";
    assert_eq!(css_at(css, &["div.p", "span.c"], "overflow-x"), "visible");
}
