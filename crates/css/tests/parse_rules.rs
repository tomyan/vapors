//! Rules (`src/parse/rules.rs`). Translated from sumi `parser/style/style_test.go` and `style_edge_test.go`; terminal properties became their CSS equivalents.

mod support;

use support::{prop, sheet};
use vapors_css::selector::{Combinator, PseudoClass};

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
fn rule_parsing_explodes_selector_lists() {
    // When
    let s = sheet("span, .hint { color: red; }");

    // Then
    assert_eq!(s.rules.len(), 2);
    for r in &s.rules {
        assert_eq!(support::prop(r, "color"), "red");
    }
}

#[test]
fn rule_parsing_populates_parsed_selector() {
    // When
    let s = sheet(".panel > span { color: red; }");

    // Then
    let sel = &s.rules[0].selector;
    assert_eq!(sel.compounds.len(), 2);
    assert_eq!(sel.combinators, [Combinator::Child]);
}

#[test]
fn rule_parsing_keeps_subject_pseudo() {
    // When
    let s = sheet(".btn:hover { color: red; }");

    // Then
    assert_eq!(
        s.rules[0].selector.compounds[0].pseudo_classes,
        [PseudoClass::Hover]
    );
}
