//! Rule and declaration parsing. Translated from sumi's
//! `parser/style/style_test.go`; sumi's terminal properties (`bold: true`,
//! `border: single`) become their CSS equivalents because vapors refuses
//! them (see `parser_refusals.rs`).

mod support;

use support::{prop, sheet};

#[test]
fn empty_stylesheet() {
    // When
    let s = sheet("");

    // Then
    assert_eq!(s.rules.len(), 0);
}

#[test]
fn whitespace_only_stylesheet() {
    // When
    let s = sheet("   \n\n\t  \n");

    // Then
    assert_eq!(s.rules.len(), 0);
}

#[test]
fn single_rule_class_selector() {
    // When
    let s = sheet(".title { color: green; }");

    // Then
    assert_eq!(s.rules.len(), 1);
    let r = &s.rules[0];
    assert_eq!(r.selector_text(), ".title");
    assert_eq!(r.declarations.len(), 1);
    assert_eq!(prop(r, "color"), "green");
}

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
fn multiple_rules() {
    // Given
    let input = ".title { color: green; }\n.subtitle { color: cyan; opacity: 0.5; }";

    // When
    let s = sheet(input);

    // Then
    assert_eq!(s.rules.len(), 2);
    assert_eq!(s.rules[0].selector_text(), ".title");
    assert_eq!(prop(&s.rules[0], "color"), "green");
    assert_eq!(s.rules[1].selector_text(), ".subtitle");
    assert_eq!(prop(&s.rules[1], "color"), "cyan");
    assert_eq!(prop(&s.rules[1], "opacity"), "0.5");
}

#[test]
fn element_selector() {
    // When
    let s = sheet("span { color: white; }");

    // Then
    assert_eq!(s.rules[0].selector_text(), "span");
    assert_eq!(prop(&s.rules[0], "color"), "white");
}

#[test]
fn box_element_selector_with_layout_properties() {
    // When
    let s = sheet("div { border-style: solid; flex-direction: row; }");

    // Then
    assert_eq!(s.rules[0].selector_text(), "div");
    assert_eq!(prop(&s.rules[0], "border-style"), "solid");
    assert_eq!(prop(&s.rules[0], "flex-direction"), "row");
}

#[test]
fn no_semicolon_on_last_property() {
    // When
    let s = sheet(".x { color: red }");

    // Then
    assert_eq!(prop(&s.rules[0], "color"), "red");
}

#[test]
fn multiline_whitespace() {
    // Given
    let input = "\n\t\t.title {\n\t\t\tcolor: green;\n\t\t\tfont-weight: bold;\n\t\t}\n\n\t\t.subtitle {\n\t\t\tcolor: cyan;\n\t\t}\n\t";

    // When
    let s = sheet(input);

    // Then
    assert_eq!(s.rules.len(), 2);
    assert_eq!(s.rules[0].selector_text(), ".title");
    assert_eq!(prop(&s.rules[0], "color"), "green");
    assert_eq!(prop(&s.rules[0], "font-weight"), "bold");
    assert_eq!(s.rules[1].selector_text(), ".subtitle");
    assert_eq!(prop(&s.rules[1], "color"), "cyan");
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
fn hover_pseudo_class_stays_in_the_selector_text() {
    // Given
    let input = ".tab:hover { color: white; opacity: 1; }";

    // When
    let s = sheet(input);

    // Then
    assert_eq!(s.rules.len(), 1);
    assert_eq!(s.rules[0].selector_text(), ".tab:hover");
    assert_eq!(prop(&s.rules[0], "color"), "white");
}

#[test]
fn base_and_hover_rules_parse_in_order() {
    // Given
    let input = ".tab { opacity: 0.5; }\n.tab:hover { opacity: 1; color: white; }";

    // When
    let s = sheet(input);

    // Then
    assert_eq!(s.rules.len(), 2);
    assert_eq!(s.rules[0].selector_text(), ".tab");
    assert_eq!(s.rules[1].selector_text(), ".tab:hover");
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
