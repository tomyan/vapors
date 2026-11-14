//! `analysis::analyse` (`src/analysis/winners.rs`, `value.rs`): custom
//! properties and `var()`, other sheets, undeclared properties, value
//! constancy and sharing.

#[path = "support/winners.rs"]
mod support;

use support::{constant, longhand, page, winners};
use vapors_css::analysis::{Constancy, Match, Sheet, Winner, analyse};

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
        ".card { width: 2em; height: 50%; color: inherit; top: unset; left: initial; background-color: currentcolor; opacity: unset; border-top-color: initial; outline-color: unset; font-weight: bolder }",
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
    // the initial border and outline colours are `currentcolor`
    assert_eq!(value("border-top-color"), Constancy::Variable);
    assert_eq!(value("outline-color"), Constancy::Variable);
    assert_eq!(value("font-weight"), Constancy::FromParent);
    assert_eq!(
        longhand(&page, "border-left-color"),
        Winner::Undeclared {
            value: Constancy::Variable
        }
    );
}

#[test]
fn two_own_sheets_of_one_origin_are_refused_naming_both() {
    // Given: an other sheet marked own
    let page = page("", ".card { color: red }", &[".card { color: blue }"]);
    let sheets: Vec<Sheet<'_>> = (page.ids.iter().zip(&page.sheets))
        .map(|(id, rules)| Sheet {
            id,
            rules,
            own: true,
        })
        .collect();

    // When
    let refusal = analyse(&sheets, &support::facts(None), &[]).expect_err("refused");

    // Then
    assert!(
        refusal.contains("\"s\"") && refusal.contains("other0"),
        "{refusal}"
    );
    assert!(refusal.contains("own: false"), "{refusal}");
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
