//! Serialize round-trips (`src/serialize.rs`). Translated from sumi `parser/style/serialize_test.go`.

mod support;

use support::prop;
use support::serialize::round_trip;

#[test]
fn serialize_round_trips_rules() {
    // When
    let s = round_trip(".title { color: red; font-weight: bold; } span { opacity: 0.5; }");

    // Then
    assert_eq!(s.rules.len(), 2);
    assert_eq!(s.rules[0].selector_text(), ".title");
    assert_eq!(prop(&s.rules[0], "color"), "red");
    assert_eq!(prop(&s.rules[1], "opacity"), "0.5");
}

#[test]
fn serialize_round_trips_pseudo_and_combinators() {
    // When
    let s =
        round_trip(".panel > span:hover { color: cyan; } .a + .b:focus { color: red !important; }");

    // Then
    assert_eq!(s.rules[0].selector_text(), ".panel > span:hover");
    assert_eq!(s.rules[1].selector_text(), ".a + .b:focus");
    assert!(s.rules[1].declarations[0].important);
}

#[test]
fn serialize_round_trips_media() {
    // When
    let s = round_trip(
        "\n.card { color: red; }\n@media (min-width: 600px) { .card { border-style: solid; } .other { color: blue; } }\n",
    );

    // Then
    assert_eq!(s.rules.len(), 3);
    assert_eq!(
        s.rules[1].media_text().as_deref(),
        Some("(min-width: 600px)")
    );
    assert_eq!(
        s.rules[2].media_text().as_deref(),
        Some("(min-width: 600px)")
    );
}

#[test]
fn serialize_round_trips_keyframes() {
    // When
    let s = round_trip(
        "@keyframes pulse { from { color: red; } 50% { color: yellow; } to { color: blue; } }",
    );

    // Then
    assert_eq!(s.keyframes.len(), 1);
    assert_eq!(s.keyframes[0].stops.len(), 3);
    assert_eq!(s.keyframes[0].stops[1].offset, 0.5);
}

#[test]
fn serialize_keeps_strings_quoted_and_functions_intact() {
    // When
    let s = round_trip(".x { font-family: \"Inter Var\", sans-serif; color: rgb(1 2 3 / 50%); }");

    // Then
    assert_eq!(
        prop(&s.rules[0], "font-family"),
        "\"Inter Var\", sans-serif"
    );
    assert_eq!(prop(&s.rules[0], "color"), "rgb(1 2 3 / 50%)");
}
