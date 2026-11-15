//! The outline each basic shape draws, as SVG 2 gives it, and the
//! numbers each refuses.

use std::collections::BTreeMap;

use super::*;

type Attributes = BTreeMap<String, String>;

fn attributes(pairs: &[(&str, &str)]) -> Attributes {
    pairs
        .iter()
        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
        .collect()
}

/// The outline of a `tag` element with `given`.
fn outline_of(tag: &str, given: &Attributes) -> Option<String> {
    outline(tag, |name| given.get(name).map(String::as_str))
}

/// Each geometry attribute's refusal for a `tag` element with `given`.
fn reasons(tag: &str, given: &Attributes) -> Vec<String> {
    (geometry_names(tag).expect("a basic shape").iter())
        .filter_map(|name| refusal(name, given.get(*name).map(String::as_str)))
        .collect()
}

#[test]
fn a_rect_draws_svgs_outline_with_its_corner_radii() {
    // Given: square corners; rx alone (ry takes it); ry alone; both;
    // radii past half the side, clamped; one radius zero, square
    let base = [("x", "1"), ("y", "2"), ("width", "10"), ("height", "6")];
    let with = |extra: &[(&'static str, &'static str)]| attributes(&[&base[..], extra].concat());
    let cases = [
        (with(&[]), "M1 2H11V8H1Z"),
        (
            with(&[("rx", "2")]),
            "M3 2H9A2 2 0 0 1 11 4V6A2 2 0 0 1 9 8H3A2 2 0 0 1 1 6V4A2 2 0 0 1 3 2Z",
        ),
        (
            with(&[("ry", "2")]),
            "M3 2H9A2 2 0 0 1 11 4V6A2 2 0 0 1 9 8H3A2 2 0 0 1 1 6V4A2 2 0 0 1 3 2Z",
        ),
        (
            with(&[("rx", "3"), ("ry", "1.5")]),
            "M4 2H8A3 1.5 0 0 1 11 3.5V6.5A3 1.5 0 0 1 8 8H4A3 1.5 0 0 1 1 6.5V3.5A3 1.5 0 0 1 4 2Z",
        ),
        (
            with(&[("rx", "9"), ("ry", "9")]),
            "M6 2H6A5 3 0 0 1 11 5V5A5 3 0 0 1 6 8H6A5 3 0 0 1 1 5V5A5 3 0 0 1 6 2Z",
        ),
        (with(&[("rx", "0"), ("ry", "2")]), "M1 2H11V8H1Z"),
    ];

    // When, Then
    for (given, expected) in cases {
        assert_eq!(
            outline_of("rect", &given).as_deref(),
            Some(expected),
            "{given:?}"
        );
    }
}

#[test]
fn a_circle_draws_two_half_arcs_from_its_leftmost_point() {
    // Given
    let circle = attributes(&[("cx", "8"), ("cy", "7"), ("r", "3")]);

    // When, Then
    assert_eq!(
        outline_of("circle", &circle).as_deref(),
        Some("M5 7A3 3 0 1 0 11 7A3 3 0 1 0 5 7Z")
    );
}

#[test]
fn a_number_svg_would_read_otherwise_or_a_size_that_draws_nothing_is_refused() {
    // Given
    let cases = [
        (
            "rect",
            vec![("width", "10px"), ("height", "6")],
            "its width \"10px\" is not a number above 0 with no unit",
        ),
        (
            "rect",
            vec![("width", "50%"), ("height", "6")],
            "its width \"50%\" is not a number above 0 with no unit",
        ),
        (
            "rect",
            vec![("height", "6")],
            "its width attribute is required",
        ),
        (
            "rect",
            vec![("width", "0"), ("height", "6")],
            "its width \"0\" is not a number above 0",
        ),
        (
            "rect",
            vec![("width", "4"), ("height", "6"), ("rx", "-1")],
            "its rx \"-1\" is not a number of 0 or more",
        ),
        (
            "rect",
            vec![("width", "4"), ("height", "6"), ("x", "1em")],
            "its x \"1em\" is not a number with no unit",
        ),
        (
            "circle",
            vec![("r", "-2")],
            "its r \"-2\" is not a number above 0",
        ),
        ("circle", vec![("cx", "8")], "its r attribute is required"),
    ];

    // When, Then
    for (tag, pairs, expected) in cases {
        let given = attributes(&pairs);
        let found = reasons(tag, &given);
        assert!(
            found.len() == 1 && found[0].starts_with(expected),
            "{tag} {pairs:?}: {found:?}"
        );
        assert_eq!(outline_of(tag, &given), None, "{tag} {pairs:?}");
    }
}

#[test]
fn a_negative_coordinate_is_taken() {
    // Given
    let rect = attributes(&[("x", "-1"), ("y", "-2.5"), ("width", "4"), ("height", "3")]);

    // When, Then
    assert_eq!(reasons("rect", &rect), Vec::<String>::new());
    assert_eq!(
        outline_of("rect", &rect).as_deref(),
        Some("M-1 -2.5H3V0.5H-1Z")
    );
}
