//! `analysis::analyse` (`src/analysis/winners.rs`, `value.rs`): custom
//! properties and `var()`, other sheets, undeclared properties, value
//! constancy and sharing.

#[path = "support/winners.rs"]
mod support;

use support::{constant, longhand, page, winners};
use vapors_css::analysis::{Constancy, Match, Winner};

#[test]
fn var_values_are_variable_and_the_custom_property_has_its_own_winner() {
    let fed_always = page("", ".card { --x: red; color: var(--x) }", &[]);
    let fed_conditional = page(
        "",
        ".card { --x: red; color: var(--x) }\n.open { --x: blue }",
        &[],
    );

    assert_eq!(
        longhand(&fed_always, "color"),
        constant(1, 0, 1, Constancy::Variable)
    );
    assert_eq!(
        winners(&fed_always).custom["--x"],
        constant(1, 0, 0, Constancy::Constant)
    );
    assert!(
        matches!(&winners(&fed_conditional).custom["--x"], Winner::Variable { overriders, .. } if *overriders == [(1, 1)])
    );
}

#[test]
fn a_rule_of_another_sheet_is_conditional_and_ties_are_run_time() {
    // Given: equal specificity in another sheet (the own rule gained the
    // scope attribute's); and one that cannot match
    let tie = page(
        "",
        ".card { color: blue }",
        &[".card[data-wisp=\"s\"] { color: red }"],
    );
    let never = page("", ".card { color: blue }", &[".other { color: red }"]);

    // Then
    let analysed = winners(&tie);
    assert!(matches!(&analysed.rules[1].verdict, Match::Conditional(d) if d.other_sheet));
    assert!(
        matches!(longhand(&tie, "color"), Winner::Variable { overriders, .. } if overriders == [(2, 0)])
    );
    assert_eq!(
        longhand(&never, "color"),
        constant(1, 0, 0, Constancy::Constant)
    );
}

#[test]
fn undeclared_properties_are_initial_or_from_the_parent() {
    let page = page("", "", &[]);
    assert_eq!(
        longhand(&page, "color"),
        Winner::Undeclared {
            value: Constancy::FromParent
        }
    );
    assert_eq!(
        longhand(&page, "width"),
        Winner::Undeclared {
            value: Constancy::Constant
        }
    );
}

#[test]
fn value_constancy_follows_units_keywords_and_colours() {
    let page = page(
        "",
        ".card { width: 2em; height: 50%; color: inherit; top: unset; left: initial; background-color: currentcolor; opacity: unset }",
        &[],
    );
    let value = |name: &str| match longhand(&page, name) {
        Winner::Constant { value, .. } => value,
        other => panic!("{name}: {other:?}"),
    };
    assert_eq!(value("width"), Constancy::Variable);
    assert_eq!(value("height"), Constancy::Variable);
    assert_eq!(value("color"), Constancy::FromParent);
    assert_eq!(value("top"), Constancy::Constant);
    assert_eq!(value("left"), Constancy::Constant);
    assert_eq!(value("background-color"), Constancy::Variable);
    assert_eq!(value("opacity"), Constancy::Constant);
}

#[test]
fn sharing_is_blocked_by_any_candidate_rule_that_reads_siblings() {
    let blocked = page(
        "",
        ".card { color: red }",
        &[".card:nth-child(2) { color: blue }"],
    );
    let open = page("", ".card { color: red }", &[".x + .other { color: blue }"]);
    assert!(!winners(&blocked).shareable);
    assert!(winners(&open).shareable);
}
