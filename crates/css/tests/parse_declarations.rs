//! Declarations (`src/parse/declarations.rs`). Translated from sumi `style_test.go` and `style_edge_test.go`, plus the refusals vapors adds.

mod support;

use support::{prop, refusal, sheet};

#[test]
fn multiple_properties() {
    // When
    let s = sheet(".title { color: green; font-weight: bold; }");

    // Then
    let r = &s.rules[0];
    assert_eq!(r.declarations.len(), 2);
    assert_eq!(prop(r, "color"), "green");
    assert_eq!(prop(r, "font-weight"), "bold");
}

#[test]
fn no_semicolon_on_last_property() {
    // When
    let s = sheet(".x { color: red }");

    // Then
    assert_eq!(prop(&s.rules[0], "color"), "red");
}

#[test]
fn property_value_with_spaces() {
    // When
    let s = sheet(".container { padding: 1px 2px; }");

    // Then
    assert_eq!(prop(&s.rules[0], "padding"), "1px 2px");
}

#[test]
fn all_properties_in_one_rule_keep_their_values() {
    // Given
    let input = ".styled {
        color: green;
        background-color: black;
        font-weight: bold;
        font-style: italic;
        text-decoration: underline;
        opacity: 0.5;
        border-style: solid;
        padding: 1px 2px 3px 4px;
        flex-direction: row;
        border-color: cyan;
    }";

    // When
    let s = sheet(input);

    // Then
    let r = &s.rules[0];
    let expected = [
        ("color", "green"),
        ("background-color", "black"),
        ("font-weight", "bold"),
        ("font-style", "italic"),
        ("text-decoration", "underline"),
        ("opacity", "0.5"),
        ("border-style", "solid"),
        ("padding", "1px 2px 3px 4px"),
        ("flex-direction", "row"),
        ("border-color", "cyan"),
    ];
    assert_eq!(r.declarations.len(), expected.len());
    for (name, want) in expected {
        assert_eq!(prop(r, name), want, "{name}");
    }
}

#[test]
fn important_flag_is_recorded_and_stripped_from_the_value() {
    // When
    let s = sheet(".x { color: red !important; width: 1px; }");

    // Then
    let r = &s.rules[0];
    assert!(r.declarations[0].important);
    assert_eq!(prop(r, "color"), "red");
    assert!(!r.declarations[1].important);
}

#[test]
fn declarations_record_their_position() {
    // When
    let s = sheet(".x {\n  color: red;\n}");

    // Then
    let d = &s.rules[0].declarations[0];
    assert_eq!((d.position.line, d.position.column), (2, 3));
}

#[test]
fn custom_properties_keep_their_raw_value() {
    // When
    let s = sheet(":root { --accent: rgb(1, 2, 3); --gap: 4px; }");

    // Then
    assert_eq!(prop(&s.rules[0], "--accent"), "rgb(1, 2, 3)");
    assert_eq!(prop(&s.rules[0], "--gap"), "4px");
}

#[test]
fn important_with_whitespace_after_the_bang_is_accepted() {
    // Given: review item 10; `! important` is valid CSS.
    let s = sheet(".x { color: red ! important; }");
    assert!(s.rules[0].declarations[0].important);
    assert_eq!(prop(&s.rules[0], "color"), "red");
}

#[test]
fn no_semicolon_multiple_properties() {
    // When
    let s = sheet(".x { color: red; font-weight: bold }");

    // Then
    assert_eq!(prop(&s.rules[0], "color"), "red");
    assert_eq!(prop(&s.rules[0], "font-weight"), "bold");
}

#[test]
fn extra_spaces_around_colon() {
    // When
    let s = sheet(".x { color :  green ; }");

    // Then
    assert_eq!(prop(&s.rules[0], "color"), "green");
}

#[test]
fn tabs_in_properties() {
    // When
    let s = sheet(".x {\n\tcolor:\tgreen;\n\tfont-weight:\tbold;\n}");

    // Then
    assert_eq!(prop(&s.rules[0], "color"), "green");
    assert_eq!(prop(&s.rules[0], "font-weight"), "bold");
}

#[test]
fn missing_colon_is_refused_naming_the_property() {
    // When
    let err = refusal(".x { color red; }");

    // Then
    assert_eq!((err.position.line, err.position.column), (1, 6));
    assert!(
        err.message.contains("color") && err.message.contains(':'),
        "{err}"
    );
}

#[test]
fn empty_value_is_refused() {
    // When
    let err = refusal(".x { color: ; }");

    // Then
    assert!(err.message.contains("color"), "{err}");
}

#[test]
fn cell_unit_is_refused_naming_the_unit() {
    // When
    let err = refusal(".x { width: 20cell; }");

    // Then
    assert_eq!(err.position.column, 13);
    assert!(
        err.message.contains("cell") && err.message.contains("px"),
        "{err}"
    );
}

#[test]
fn vendor_prefixed_properties_on_the_allowlist_are_dropped() {
    // When
    let s = sheet(".x { -webkit-font-smoothing: antialiased; color: red; }");

    // Then
    assert_eq!(s.rules[0].declarations.len(), 1);
    assert_eq!(s.rules[0].declarations[0].name, "color");
}

#[test]
fn vendor_prefixed_properties_off_the_allowlist_are_refused() {
    // When
    let err = refusal(".x { -moz-appearance: none; }");

    // Then
    assert!(err.message.contains("-moz-appearance"), "{err}");
}
