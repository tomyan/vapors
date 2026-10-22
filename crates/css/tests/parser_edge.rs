//! Comments, malformed input and whitespace. Translated from sumi's
//! `parser/style/style_edge_test.go`; where sumi only checks that an error
//! occurred, vapors also checks the error names the line and column.

mod support;

use support::{prop, refusal, sheet};

#[test]
fn comments() {
    // Given
    let input =
        "/* header comment */\n.title {\n\t/* color is green */\n\tcolor: green;\n}\n/* footer */";

    // When
    let s = sheet(input);

    // Then
    assert_eq!(s.rules.len(), 1);
    assert_eq!(prop(&s.rules[0], "color"), "green");
}

#[test]
fn unterminated_block_is_refused_at_the_open_brace() {
    // When
    let err = refusal(".title { color: green;");

    // Then
    assert_eq!((err.position.line, err.position.column), (1, 8));
    assert!(err.message.contains('}'), "{err}");
}

#[test]
fn missing_selector_before_brace_is_refused() {
    // When
    let err = refusal("{ color: green; }");

    // Then
    assert_eq!((err.position.line, err.position.column), (1, 1));
    assert!(err.message.contains("selector"), "{err}");
}

#[test]
fn phase4_demo_stylesheet() {
    // Given
    let input = ".container {\n\tborder-style: solid;\n\tpadding: 1px 2px;\n}\n.title {\n\tcolor: green;\n\tfont-weight: bold;\n}\n.subtitle {\n\tcolor: cyan;\n\topacity: 0.5;\n}\n.count {\n\tcolor: yellow;\n\tfont-weight: bold;\n}";

    // When
    let s = sheet(input);

    // Then
    assert_eq!(s.rules.len(), 4);
    assert_eq!(s.rules[0].selector_text(), ".container");
    assert_eq!(prop(&s.rules[0], "border-style"), "solid");
    assert_eq!(prop(&s.rules[0], "padding"), "1px 2px");
    assert_eq!(s.rules[1].selector_text(), ".title");
    assert_eq!(prop(&s.rules[1], "color"), "green");
    assert_eq!(prop(&s.rules[1], "font-weight"), "bold");
    assert_eq!(s.rules[2].selector_text(), ".subtitle");
    assert_eq!(prop(&s.rules[2], "opacity"), "0.5");
    assert_eq!(s.rules[3].selector_text(), ".count");
    assert_eq!(prop(&s.rules[3], "color"), "yellow");
}

#[test]
fn mixed_element_and_class_selectors() {
    // When
    let s = sheet("span { color: white; }\n.highlight { font-weight: bold; }");

    // Then
    assert_eq!(s.rules[0].selector_text(), "span");
    assert_eq!(s.rules[1].selector_text(), ".highlight");
}

#[test]
fn unterminated_comment_is_refused() {
    // When
    let err = refusal("/* unterminated comment .title { color: green; }");

    // Then
    assert_eq!((err.position.line, err.position.column), (1, 1));
    assert!(err.message.contains("comment"), "{err}");
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
fn stray_close_brace_is_refused() {
    // When
    let err = refusal(".x { color: red; } }");

    // Then
    assert_eq!(err.position.column, 20);
}
