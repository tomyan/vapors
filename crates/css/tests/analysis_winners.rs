//! `analysis::analyse` (`src/analysis/winners.rs`, `standing.rs`,
//! `value.rs`): per longhand and custom property, a constant or
//! variable winner, and a constant, variable or parent-taken value, as
//! `build-time-styles.md` section 1 classifies them.

use vapors_css::analysis::{Constancy, KeyFacts, Match, Place, Sheet, SheetId, Winner, analyse};
use vapors_css::cascade::INLINE_SHEET;
use vapors_css::property::PropertyId;

/// `<div class="card" class:open={..} data-wisp="s">`, optionally with
/// a literal `style`.
fn facts(style: Option<&'static str>) -> KeyFacts<'static> {
    KeyFacts {
        tag: "div",
        scope_attribute: "data-wisp",
        scope: Some("s"),
        classes: &["card"],
        toggles: &["open"],
        style,
        literal_attributes: &[],
        expression_attributes: &[],
    }
}

/// The sheets as the compiler builds them: UA, the own sheet scoped
/// as `s`, then every other sheet; `style` is the literal attribute.
struct Page {
    ids: Vec<SheetId>,
    sheets: Vec<vapors_css::ast::Stylesheet>,
    style: Option<&'static str>,
}

fn page(ua: &str, own: &str, others: &[&str]) -> Page {
    let scoped = vapors_css::scope::scoped(vapors_css::parse(own).expect("own"), "data-wisp", "s");
    let mut ids = vec![SheetId::UserAgent, SheetId::Scope("s".into())];
    let mut sheets = vec![vapors_css::parse(ua).expect("ua"), scoped];
    for (index, other) in others.iter().enumerate() {
        ids.push(SheetId::Scope(format!("other{index}")));
        sheets.push(vapors_css::parse(other).expect("other"));
    }
    Page {
        ids,
        sheets,
        style: None,
    }
}

fn winners(page: &Page) -> vapors_css::analysis::KeyWinners {
    let sheets: Vec<Sheet<'_>> = (page.ids.iter().zip(&page.sheets).enumerate())
        .map(|(index, (id, rules))| Sheet {
            id,
            rules,
            own: index < 2,
        })
        .collect();
    let inline = page
        .style
        .map(|style| vapors_css::parse_inline(style).expect("inline"))
        .unwrap_or_default();
    analyse(&sheets, &facts(page.style), &inline)
}

fn longhand(page: &Page, name: &str) -> Winner {
    winners(page).longhands[PropertyId::from_name(name).expect("longhand").index()].clone()
}

fn constant(sheet: usize, rule: usize, declaration: usize, value: Constancy) -> Winner {
    Winner::Constant {
        place: Place {
            sheet,
            rule,
            declaration,
        },
        value,
    }
}

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
        constant(1, 1, 0, Constancy::Constant)
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
