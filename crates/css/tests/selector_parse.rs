//! Selector parsing. Translated from sumi's `parser/style/selector_test.go`,
//! `selector_attr_test.go`, `selector_structural_test.go` and `nth_test.go`.

mod support;

use support::{selector, sheet};
use vapors_css::selector::{
    AttributeOperator, Combinator, Nth, PseudoClass, Specificity, parse_selector_list,
};

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
fn specificity_ordering() {
    for (lower, higher) in [
        ("span", ".title"),
        (".title", "#main"),
        (".a", ".a.b"),
        ("span", "div span"),
        ("*", "span"),
    ] {
        // When / Then
        assert!(
            selector(lower).specificity() < selector(higher).specificity(),
            "specificity({lower}) should be < specificity({higher})"
        );
    }
}

#[test]
fn specificity_is_the_id_class_type_triple() {
    // When / Then
    assert_eq!(
        selector("div#a.b.c[d]:hover").specificity(),
        Specificity {
            ids: 1,
            classes: 4,
            types: 1
        }
    );
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

// --- attributes ---

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

#[test]
fn attribute_selector_specificity_counts_as_class() {
    assert!(selector("div").specificity() < selector("[focusable]").specificity());
}

// --- structural ---

#[test]
fn parse_structural_pseudo_separated_from_state() {
    // When
    let c = &selector("span:first-child:hover").compounds[0];

    // Then
    assert_eq!(
        c.pseudo_classes,
        [PseudoClass::FirstChild, PseudoClass::Hover]
    );
}

#[test]
fn parse_nth_child_argument() {
    // When
    let c = &selector("span:nth-child(2n+1)").compounds[0];

    // Then
    assert_eq!(
        c.pseudo_classes,
        [PseudoClass::NthChild(Nth { a: 2, b: 1 })]
    );
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
fn structural_pseudo_counts_in_specificity() {
    assert!(selector("span").specificity() < selector("span:first-child").specificity());
}

// --- An+B ---

#[test]
fn parse_nth_keywords() {
    for (expr, a, b) in [
        ("odd", 2, 1),
        ("even", 2, 0),
        ("3", 0, 3),
        ("n", 1, 0),
        ("2n", 2, 0),
        ("2n+1", 2, 1),
        ("-n+3", -1, 3),
        ("3n-1", 3, -1),
        ("+2n+2", 2, 2),
        ("2n + 1", 2, 1),
        ("-2n - 3", -2, -3),
    ] {
        // When
        let nth = Nth::parse(expr).unwrap_or_else(|e| panic!("{expr}: {e}"));

        // Then
        assert_eq!(nth, Nth { a, b }, "{expr}");
    }
}

#[test]
fn nth_matches() {
    for (expr, index, want) in [
        ("odd", 1, true),
        ("odd", 2, false),
        ("even", 2, true),
        ("3", 3, true),
        ("3", 4, false),
        ("2n+1", 5, true),
        ("-n+3", 3, true),
        ("-n+3", 4, false),
        ("n", 7, true),
    ] {
        // When
        let nth = Nth::parse(expr).expect(expr);

        // Then
        assert_eq!(nth.matches(index), want, "{expr}.matches({index})");
    }
}

#[test]
fn parse_nth_invalid() {
    for expr in ["", "x", "n+n", "2m+1", "2 n"] {
        assert!(Nth::parse(expr).is_err(), "{expr:?} should be refused");
    }
}
