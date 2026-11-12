//! What a tree sharing computed styles compares beyond the template
//! key (analysis PR 2): the attribute names and state flags selectors
//! read, and the rules a key's elements can match.

use std::collections::BTreeSet;

use vapors_css::cascade::Cascade;
use vapors_css::selector::{attribute_names, parse_selector_list, state_flags};
use vapors_css::tree::States;

fn selector(text: &str) -> vapors_css::selector::Selector {
    parse_selector_list(text).expect("parses").remove(0)
}

fn cascade(sheet: &str) -> Cascade {
    Cascade::new().author(vapors_css::parse(sheet).expect("parses"))
}

#[test]
fn attribute_names_are_every_attribute_selected_and_id_for_an_id_selector() {
    // Given
    let mut names = BTreeSet::new();

    // When
    for text in [
        "input[type=text]:not([disabled]) .x",
        "#main .a",
        ".b:is([hidden])",
        ".c:has([role])",
        ".d:where([lang])",
        ":global(#g) .e",
    ] {
        attribute_names(&selector(text), &mut names);
    }

    // Then
    let expected: BTreeSet<String> = ["type", "disabled", "id", "hidden", "role", "lang"]
        .map(str::to_owned)
        .into_iter()
        .collect();
    assert_eq!(names, expected);
}

#[test]
fn state_flags_are_every_state_tested_anywhere_in_the_selector() {
    // Given
    let cases = [
        (".a:hover > .b", States::HOVER),
        (".a:not(:focus-within)", States::FOCUS_WITHIN),
        (".a:has(:checked)", States::CHECKED),
        (
            ":where(:disabled, :invalid) .a",
            States::DISABLED.union(States::INVALID),
        ),
        (".a:first-child[x]", States::NONE),
    ];

    for (text, expected) in cases {
        // Then
        assert_eq!(state_flags(&selector(text)), expected, "{text}");
    }
}

#[test]
fn a_cascade_knows_the_attribute_names_and_states_its_sheets_select() {
    // Given
    let cascade = Cascade::new()
        .user_agent(
            vapors_css::parse("[hidden] { display: none } #x:focus { color: red }")
                .expect("parses"),
        )
        .author(vapors_css::parse(".a:hover { color: blue }").expect("parses"));

    // Then
    let names: Vec<&str> = cascade
        .selected_attributes()
        .iter()
        .map(String::as_str)
        .collect();
    assert_eq!(names, ["hidden", "id"]);
    assert_eq!(
        cascade.selected_states(),
        States::FOCUS.union(States::HOVER)
    );
}

fn preludes(cascade: &Cascade, tag: &str, classes: &[&str]) -> Vec<String> {
    (cascade.rules_for_key(tag, classes))
        .map(|rule| {
            vapors_css::serialize::component_values(&rule.prelude)
                .trim()
                .to_owned()
        })
        .collect()
}

#[test]
fn the_rules_for_a_key_are_its_tag_class_universal_and_every_id_bucket() {
    // Given
    let cascade = cascade(".row { } #x { } span { } * { } .other { } div .row { } div { }");

    // Then
    assert_eq!(
        preludes(&cascade, "div", &["row", "marked"]),
        [".row", "#x", "*", "div .row", "div"]
    );
}

#[test]
fn the_rules_for_a_key_find_its_tag_whatever_its_case() {
    // Given
    let cascade = cascade("div { } span { }");

    // Then
    assert_eq!(preludes(&cascade, "DIV", &[]), ["div"]);
}
