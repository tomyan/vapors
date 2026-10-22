//! Attribute selectors (`src/selector/attribute.rs`). Translated from sumi `selector_attr_test.go` and `resolve_attr_test.go`.

mod support;

use support::{matches_chain, selector};
use vapors_css::selector::{AttributeOperator, matches};
use vapors_css::tree::Tree;

#[test]
fn attribute_presence() {
    assert!(matches_chain("[focusable]", &["div[focusable=true]"]));
    assert!(!matches_chain("[focusable]", &["div"]));
}

#[test]
fn attribute_operators() {
    for (sel, value, want) in [
        ("[a=hello]", "hello", true),
        ("[a=hello]", "hell", false),
        ("[a^=he]", "hello", true),
        ("[a^=lo]", "hello", false),
        ("[a$=lo]", "hello", true),
        ("[a$=he]", "hello", false),
        ("[a*=ell]", "hello", true),
        ("[a*=xyz]", "hello", false),
        ("[a~=b]", "a b c", true),
        ("[a~=d]", "a b c", false),
        ("[a|=en]", "en-GB", true),
        ("[a|=en]", "en", true),
        ("[a|=en]", "enx", false),
        ("[a=HELLO i]", "hello", true),
        ("[a=HELLO]", "hello", false),
    ] {
        // Given
        let mut tree = Tree::new("root");
        let root = tree.root();
        let el = tree.append(root, "div");
        tree.set_attribute(el, "a", value);

        // When / Then
        assert_eq!(
            matches(&selector(sel), &tree.node(el)),
            want,
            "{sel} vs {value:?}"
        );
    }
}

#[test]
fn parse_attribute_presence() {
    // When
    let attrs = &selector("[focusable]").compounds[0].attributes;

    // Then
    assert_eq!(attrs.len(), 1);
    assert_eq!(attrs[0].name, "focusable");
    assert_eq!(attrs[0].operator, AttributeOperator::Exists);
}

#[test]
fn parse_attribute_operators() {
    for (sel, op) in [
        ("[a=v]", AttributeOperator::Equals),
        ("[a^=v]", AttributeOperator::Prefix),
        ("[a$=v]", AttributeOperator::Suffix),
        ("[a*=v]", AttributeOperator::Substring),
        ("[a~=v]", AttributeOperator::Includes),
        ("[a|=v]", AttributeOperator::DashMatch),
    ] {
        // When
        let attrs = &selector(sel).compounds[0].attributes;

        // Then
        assert_eq!(attrs[0].operator, op, "{sel}");
        assert_eq!(attrs[0].value, "v", "{sel}");
    }
}

#[test]
fn parse_attribute_quoted_value() {
    // When
    let attrs = &selector(r#"[title="hello"]"#).compounds[0].attributes;

    // Then
    assert_eq!(attrs[0].value, "hello");
}

#[test]
fn parse_attribute_case_insensitive_flag() {
    // When
    let attrs = &selector(r#"[title="hello" i]"#).compounds[0].attributes;

    // Then
    assert!(attrs[0].case_insensitive);
}

#[test]
fn parse_attribute_on_compound() {
    // When
    let c = &selector("div.panel[focusable=true]").compounds[0];

    // Then
    assert_eq!(c.tag.as_deref(), Some("div"));
    assert_eq!((c.classes.len(), c.attributes.len()), (1, 1));
}
