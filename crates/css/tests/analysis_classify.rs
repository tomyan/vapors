//! `analysis::classify` (`src/analysis/`): one rule against a template
//! key's fixed parts, per selector shape, as `build-time-styles.md`
//! section 1 classifies them. The soundness of `Always` and `Never` is
//! checked against the matcher in `analysis_oracle.rs`.

use vapors_css::analysis::{Depends, KeyFacts, Match, classify};
use vapors_css::tree::States;

/// `<button class="card" class:open={..} type="text" id={..} data-wisp="s">`
const CARD: KeyFacts<'static> = KeyFacts {
    tag: "button",
    scope_attribute: "data-wisp",
    scope: Some("s"),
    classes: &["card"],
    toggles: &["open"],
    style: Some("color: red"),
    literal_attributes: &[("type", "Text")],
};

/// `css` with its subject scoped as `scope::scoped` scopes a component's
/// sheet for scope `s`.
fn scoped(css: &str) -> String {
    let sheet = vapors_css::parse(css).unwrap_or_else(|e| panic!("{css}: {e}"));
    vapors_css::scope::scoped(sheet, "data-wisp", "s").to_css()
}

fn classified(css: &str, facts: &KeyFacts<'_>) -> Match {
    let sheet = vapors_css::parse(css).unwrap_or_else(|e| panic!("{css}: {e}"));
    classify(&sheet.rules[0], facts)
}

fn depends(set: impl FnOnce(&mut Depends)) -> Match {
    let mut depends = Depends::default();
    set(&mut depends);
    Match::Conditional(depends)
}

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
