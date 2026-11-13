//! `analysis::classify` over rules in the form `scope::scoped` gives
//! them, the state pseudo-class names `Depends` prints, and its text.

#[path = "support/classify.rs"]
mod support;

use support::{CARD, classified, depends, scoped};
use vapors_css::analysis::{KeyFacts, Match};

#[test]
fn an_unscoped_key_never_matches_a_scoped_rule() {
    let unscoped = KeyFacts {
        scope: None,
        ..CARD
    };
    assert_eq!(
        classified(&scoped("button { color: red }"), &unscoped),
        Match::Never
    );
}

#[test]
fn scoped_rules_are_judged_in_the_form_the_runtime_scopes_them() {
    // Given: the subject gains the attribute bare, the rest inside :where()
    assert_eq!(classified(&scoped(".card { color: red }"), &CARD), Match::Always);
    assert_eq!(classified(&scoped(".other { color: red }"), &CARD), Match::Never);
    assert_eq!(
        classified(&scoped(":global(.dark) .card { color: red }"), &CARD),
        depends(|d| d.ancestors = true)
    );
    assert_eq!(
        classified(&scoped(".panel :global(button) { color: red }"), &CARD),
        depends(|d| d.ancestors = true)
    );
    assert_eq!(
        classified(&scoped(":is(.open, .card) { color: red }"), &CARD),
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
