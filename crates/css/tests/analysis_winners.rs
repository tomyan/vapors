//! `analysis::analyse` (`src/analysis/winners.rs`, `standing.rs`): the
//! winner per longhand under the cascade's standing, as
//! `build-time-styles.md` section 1 classifies it. Values and sharing:
//! `analysis_winners_values.rs`.

#[path = "support/winners.rs"]
mod support;

use support::{constant, longhand, page};
use vapors_css::analysis::{Constancy, Place, Winner};
use vapors_css::cascade::INLINE_SHEET;

#[test]
fn an_always_rule_of_the_own_sheet_is_a_constant_winner_with_a_constant_value() {
    let page = page("", ".card { color: red; width: 4px }", &[]);
    assert_eq!(
        longhand(&page, "color"),
        constant(1, 0, 0, Constancy::Constant)
    );
    assert_eq!(
        longhand(&page, "width"),
        constant(1, 0, 1, Constancy::Constant)
    );
}

#[test]
fn a_later_conditional_rule_of_equal_specificity_overrides_and_an_earlier_one_does_not() {
    // Given: `.open` and `.card` both weigh (0,1,0) scoped
    let later = page("", ".card { color: red }\n.open { color: blue }", &[]);
    let earlier = page("", ".open { color: blue }\n.card { color: red }", &[]);

    // Then
    assert_eq!(
        longhand(&later, "color"),
        Winner::Variable {
            base: Some(Place {
                sheet: 1,
                rule: 0,
                declaration: 0
            }),
            overriders: vec![(1, 1)],
        }
    );
    assert_eq!(
        longhand(&earlier, "color"),
        constant(1, 1, 0, Constancy::Constant)
    );
}

#[test]
fn specificity_decides_before_order() {
    // Given: `div.card:hover` (0,2,1) beats `.card` (0,1,0); `:where(.open)` (0,0,0) does not
    let higher = page(
        "",
        "div.card:hover { color: blue }\n.card { color: red }",
        &[],
    );
    let lower = page(
        "",
        ".card { color: red }\n:where(.open) { color: blue }",
        &[],
    );

    // Then
    assert!(
        matches!(longhand(&higher, "color"), Winner::Variable { overriders, .. } if overriders == [(1, 0)])
    );
    assert_eq!(
        longhand(&lower, "color"),
        constant(1, 0, 0, Constancy::Constant)
    );
}

#[test]
fn importance_ranks_across_origins_as_the_cascade_does() {
    // Given: UA important beats author important; author normal beats UA normal
    let ua_wins = page(
        "div { color: red !important }",
        ".card { color: blue !important }",
        &[],
    );
    let author_wins = page("div { color: red }", ".card { color: blue }", &[]);
    let conditional_important = page(
        "",
        ".card { color: red }\n:where(.open) { color: blue !important }",
        &[],
    );

    // Then
    assert_eq!(
        longhand(&ua_wins, "color"),
        constant(0, 0, 0, Constancy::Constant)
    );
    assert_eq!(
        longhand(&author_wins, "color"),
        constant(1, 0, 0, Constancy::Constant)
    );
    assert!(
        matches!(longhand(&conditional_important, "color"), Winner::Variable { overriders, .. } if overriders == [(1, 1)])
    );
}

#[test]
fn the_literal_style_beats_every_normal_rule_and_loses_to_an_important_conditional_one() {
    let mut beaten = page("", "div.card { color: red }", &[]);
    beaten.style = Some("color: green");
    let mut overridden = page("", ".open { color: red !important }", &[]);
    overridden.style = Some("color: green");

    assert_eq!(
        longhand(&beaten, "color"),
        constant(INLINE_SHEET, 0, 0, Constancy::Constant)
    );
    assert!(
        matches!(longhand(&overridden, "color"), Winner::Variable { base: Some(Place { sheet: INLINE_SHEET, .. }), overriders } if overriders == [(1, 0)])
    );
}

#[test]
fn a_shorthand_in_a_conditional_rule_overrides_an_always_longhand() {
    let page = page("", ".card { margin-top: 1px }\n.open { margin: 2px }", &[]);
    assert!(
        matches!(longhand(&page, "margin-top"), Winner::Variable { base: Some(_), overriders } if overriders == [(1, 1)])
    );
    assert!(
        matches!(longhand(&page, "margin-left"), Winner::Variable { base: None, overriders } if overriders == [(1, 1)])
    );
}

#[test]
fn the_all_property_is_refused_at_parse_so_the_analysis_never_sees_it() {
    let error = vapors_css::parse(".card { all: unset }").expect_err("refused");
    assert!(error.message.contains("all"), "{error}");
}
