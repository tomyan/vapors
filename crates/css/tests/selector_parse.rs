//! Selector parsing (`src/selector/parse.rs`). Translated from sumi `selector_test.go` and `selector_structural_test.go`.

mod support;

use support::selector;
use vapors_css::selector::{Combinator, Nth, PseudoClass, parse_selector_list};

#[test]
fn parse_selector_simple_class() {
    // When
    let s = selector(".title");

    // Then
    assert_eq!(s.compounds.len(), 1);
    let c = &s.compounds[0];
    assert_eq!(c.tag, None);
    assert_eq!(c.classes, ["title"]);
}

#[test]
fn parse_selector_tag_with_class_and_id() {
    // When
    let c = &selector("div.panel#main").compounds[0];

    // Then
    assert_eq!(c.tag.as_deref(), Some("div"));
    assert_eq!(c.id.as_deref(), Some("main"));
    assert_eq!(c.classes, ["panel"]);
}

#[test]
fn parse_selector_universal() {
    // When
    let c = &selector("*").compounds[0];

    // Then
    assert!(c.tag.is_none() && c.id.is_none() && c.classes.is_empty());
}

#[test]
fn parse_selector_descendant() {
    // When
    let s = selector(".panel span");

    // Then
    assert_eq!(s.compounds.len(), 2);
    assert_eq!(s.combinators, [Combinator::Descendant]);
    assert_eq!(s.compounds[1].tag.as_deref(), Some("span"));
}

#[test]
fn parse_selector_child() {
    // When
    let s = selector(".panel > .row");

    // Then
    assert_eq!(s.compounds.len(), 2);
    assert_eq!(s.combinators, [Combinator::Child]);
}

#[test]
fn parse_selector_list_of_two() {
    // When
    let list = parse_selector_list("span, .hint").expect("parses");

    // Then
    assert_eq!(list.len(), 2);
}

#[test]
fn parse_selector_pseudo_on_subject() {
    // When
    let s = selector(".panel .btn:hover");

    // Then
    assert_eq!(s.compounds[1].pseudo_classes, [PseudoClass::Hover]);
    assert_eq!(s.compounds[0].pseudo_classes, []);
}

#[test]
fn parse_sibling_combinators() {
    for (sel, comb) in [
        (".a + .b", Combinator::NextSibling),
        (".a ~ .b", Combinator::SubsequentSibling),
        (".a+.b", Combinator::NextSibling),
        (".a~.b", Combinator::SubsequentSibling),
        (".a>.b", Combinator::Child),
    ] {
        // When
        let s = selector(sel);

        // Then
        assert_eq!(s.compounds.len(), 2, "{sel}");
        assert_eq!(s.combinators, [comb], "{sel}");
    }
}

#[test]
fn parse_plus_inside_nth_not_a_combinator() {
    // When
    let s = selector(".list :nth-child(2n+1)");

    // Then
    assert_eq!(s.compounds.len(), 2);
    assert_eq!(s.combinators, [Combinator::Descendant]);
    assert_eq!(
        s.compounds[1].pseudo_classes,
        [PseudoClass::NthChild(Nth { a: 2, b: 1 })]
    );
}

#[test]
fn malformed_selectors_are_refused_with_a_position() {
    for (text, column) in [
        (".", 1),
        ("div >", 5),
        ("> div", 1),
        ("[a", 1),
        ("[=v]", 1),
        ("div:nth-child(2m)", 4),
        ("div:not()", 4),
        ("a,,b", 3),
        (":has(:has(a))", 6),
    ] {
        let err = parse_selector_list(text).expect_err(text);
        assert_eq!(err.position.column, column, "{text}: {err}");
    }
}
