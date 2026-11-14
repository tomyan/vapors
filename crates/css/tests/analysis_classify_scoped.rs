//! `analysis::classify` over rules in the form `scope::scoped` gives
//! them, the state pseudo-class names `Depends` prints, and its text.

#[path = "support/classify.rs"]
mod support;

use support::{CARD, classified, classified_scoped, depends};
use vapors_css::analysis::{KeyFacts, Match};

#[test]
fn an_unscoped_key_never_matches_a_scoped_rule() {
    let unscoped = KeyFacts {
        scope: None,
        ..CARD
    };
    assert_eq!(
        classified_scoped("button { color: red }", &unscoped),
        Match::Never
    );
}

#[test]
fn scoped_rules_are_judged_in_the_form_the_runtime_scopes_them() {
    // Given: the subject gains the attribute bare, the rest inside :where()
    assert_eq!(
        classified_scoped(".card { color: red }", &CARD),
        Match::Always
    );
    assert_eq!(
        classified_scoped(".other { color: red }", &CARD),
        Match::Never
    );
    assert_eq!(
        classified_scoped(":global(.dark) .card { color: red }", &CARD),
        depends(|d| d.ancestors = true)
    );
    assert_eq!(
        classified_scoped(".panel :global(button) { color: red }", &CARD),
        depends(|d| d.ancestors = true)
    );
    assert_eq!(
        classified_scoped(":is(.open, .card) { color: red }", &CARD),
        Match::Always
    );
}

#[test]
fn state_names_round_trip_through_every_state_pseudo_class() {
    for name in [
        ":hover",
        ":focus",
        ":focus-visible",
        ":focus-within",
        ":active",
        ":checked",
        ":disabled",
        ":enabled",
        ":invalid",
    ] {
        let Match::Conditional(depends) = classified(&format!("{name} {{ color: red }}"), &CARD)
        else {
            panic!("{name}: conditional");
        };
        assert_eq!(depends.to_string(), name);
    }
}

#[test]
fn dependencies_display_as_a_list() {
    let Match::Conditional(depends) = classified(
        "@media print { .panel .open:hover[hidden] { color: red } }",
        &CARD,
    ) else {
        panic!("conditional");
    };
    assert_eq!(
        depends.to_string(),
        "class:open, :hover, [hidden], ancestors, @media/@supports/@container"
    );
}

#[test]
#[should_panic(expected = "a directive is also a literal class")]
fn a_directive_that_is_also_a_literal_class_fails_the_shape_check() {
    let facts = KeyFacts {
        classes: &["card"],
        toggles: &["card"],
        ..CARD
    };
    classified("div { color: red }", &facts);
}

#[test]
#[should_panic(expected = "the key's own fields")]
fn class_style_or_the_scope_attribute_under_literal_attributes_fails_the_shape_check() {
    let facts = KeyFacts {
        literal_attributes: &[("Style", "color: red")],
        ..CARD
    };
    classified("div { color: red }", &facts);
}

#[test]
#[should_panic(expected = "an attribute is listed twice")]
fn an_attribute_listed_twice_in_any_case_fails_the_shape_check() {
    let facts = KeyFacts {
        literal_attributes: &[("title", "a"), ("Title", "b")],
        ..CARD
    };
    classified("div { color: red }", &facts);
}

#[test]
#[should_panic(expected = "the key's own fields")]
fn the_scope_attribute_in_another_case_fails_the_shape_check() {
    let facts = KeyFacts {
        literal_attributes: &[("DATA-WISP", "s")],
        ..CARD
    };
    classified("div { color: red }", &facts);
}
