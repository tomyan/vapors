//! `analysis::classify` (`src/analysis/`): one rule against a template
//! key's fixed parts, per selector shape, as `build-time-styles.md`
//! section 1 classifies them. The soundness of `Always` and `Never` is
//! checked against the matcher in `analysis_oracle.rs`; the scoped
//! forms, state names and `Depends` text are in
//! `analysis_classify_scoped.rs`.

#[path = "support/classify.rs"]
mod support;

use support::{CARD, classified, depends};
use vapors_css::analysis::{KeyFacts, Match};
use vapors_css::tree::States;

#[test]
fn always_from_the_fixed_parts_alone() {
    for css in [
        "button { color: red }",
        "BUTTON { color: red }",
        ".card { color: red }",
        "button.card[data-wisp=\"s\"] { color: red }",
        "[type=text i] { color: red }",
        "[type=\"Text\"] { color: red }",
        "[style] { color: red }",
        ":is(.card, .other) { color: red }",
        ":not(.other) { color: red }",
        ":not(div) { color: red }",
        "* { color: red }",
    ] {
        assert_eq!(classified(css, &CARD), Match::Always, "{css}");
    }
}

#[test]
fn never_when_a_fixed_part_rules_it_out() {
    for css in [
        "div { color: red }",
        ".other { color: red }",
        "button.card.other { color: red }",
        "[data-wisp=\"t\"] { color: red }",
        "[type=number] { color: red }",
        "[type=text] { color: red }",
        "[data-x].other { color: red }",
        ":not(.card) { color: red }",
        ":not(:is(button, div)) { color: red }",
        "[class] div { color: red }",
    ] {
        assert_eq!(classified(css, &CARD), Match::Never, "{css}");
    }
}

#[test]
fn a_directive_class_is_conditional_on_that_directive() {
    let expected = depends(|d| {
        d.toggles.insert("open".into());
    });
    assert_eq!(classified(".open { color: red }", &CARD), expected);
    assert_eq!(classified(":not(.open) { color: red }", &CARD), expected);
    assert_eq!(
        classified("button.card.open { color: red }", &CARD),
        expected
    );
}

#[test]
fn a_state_is_conditional_on_its_flag() {
    assert_eq!(
        classified("button:hover { color: red }", &CARD),
        depends(|d| d.states = States::HOVER)
    );
    assert_eq!(
        classified(".card:focus-visible:active { color: red }", &CARD),
        depends(|d| d.states = States::FOCUS_VISIBLE.union(States::ACTIVE))
    );
}

#[test]
fn run_time_attributes_are_conditional_on_their_name() {
    // Given: `id` and `hidden` are not literal in the template
    for (css, name) in [
        ("#main { color: red }", "id"),
        ("[id] { color: red }", "id"),
        ("[hidden] { color: red }", "hidden"),
        ("[aria-pressed=true] { color: red }", "aria-pressed"),
    ] {
        assert_eq!(
            classified(css, &CARD),
            depends(|d| {
                d.attributes.insert(name.into());
            }),
            "{css}"
        );
    }
}

#[test]
fn the_class_attribute_as_text_is_left_to_run_time_when_the_key_sets_one() {
    assert_eq!(
        classified("[class~=card] { color: red }", &CARD),
        depends(|d| {
            d.toggles.insert("open".into());
            d.attributes.insert("class".into());
        })
    );
    let bare = KeyFacts {
        classes: &[],
        toggles: &[],
        ..CARD
    };
    assert_eq!(classified("[class] { color: red }", &bare), Match::Never);
}

#[test]
fn structure_and_ancestors_are_conditional() {
    for (css, set) in [
        (":first-child { color: red }", "structure"),
        (":empty { color: red }", "structure"),
        (":nth-child(2n) { color: red }", "structure"),
        (":has(> span) { color: red }", "structure"),
        (".a + button { color: red }", "structure"),
        (":root { color: red }", "ancestors"),
        (".panel button { color: red }", "ancestors"),
        (".panel > .card { color: red }", "ancestors"),
        (":global(.dark) .card { color: red }", "ancestors"),
    ] {
        let expected = depends(|d| match set {
            "structure" => d.structure = true,
            _ => d.ancestors = true,
        });
        assert_eq!(classified(css, &CARD), expected, "{css}");
    }
}

#[test]
fn conditions_make_an_always_rule_conditional_and_leave_never_alone() {
    assert_eq!(
        classified("@media (min-width: 10px) { button { color: red } }", &CARD),
        depends(|d| d.conditions = true)
    );
    assert_eq!(
        classified("@media (min-width: 10px) { div { color: red } }", &CARD),
        Match::Never
    );
}

#[test]
fn global_and_nested_lists_combine_their_arguments() {
    assert_eq!(
        classified(":global(button) { color: red }", &CARD),
        Match::Always
    );
    assert_eq!(
        classified(":is(.open, div) { color: red }", &CARD),
        depends(|d| {
            d.toggles.insert("open".into());
        })
    );
    assert_eq!(
        classified(":not(.open, .card) { color: red }", &CARD),
        Match::Never
    );
}
