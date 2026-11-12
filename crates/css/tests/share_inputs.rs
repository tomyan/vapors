//! What a computed style can be shared on (analysis PR 2): whether a
//! selector reads an element's siblings or descendants, which attribute
//! names selectors read, and the rules a template key's elements can
//! match.

use std::collections::BTreeSet;

use vapors_css::cascade::Cascade;
use vapors_css::selector::{attribute_names, parse_selector_list, reads_siblings_or_descendants};

fn selector(text: &str) -> vapors_css::selector::Selector {
    parse_selector_list(text).expect("parses").remove(0)
}

#[test]
fn ancestor_combinators_and_state_pseudo_classes_read_only_the_chain() {
    for text in [
        ".a .b",
        ".a > .b",
        "div.a:hover > .b:focus-within",
        ".a:is(.b, .c)",
        ".a:not(.b)",
        "input[type=text]",
        ":root .a",
    ] {
        assert!(!reads_siblings_or_descendants(&selector(text)), "{text}");
    }
}

#[test]
fn sibling_combinators_positions_empty_and_has_read_beyond_the_chain() {
    for text in [
        ".a + .b",
        ".a ~ .b .c",
        ".a:first-child",
        "li:nth-of-type(2)",
        ".a:last-child > .b",
        ".a:empty",
        ".a:has(.b)",
        ".a:not(.b + .c)",
        ".a:is(.b:only-child)",
    ] {
        assert!(reads_siblings_or_descendants(&selector(text)), "{text}");
    }
}

#[test]
fn attribute_names_are_every_attribute_selected_and_id_for_an_id_selector() {
    // Given
    let mut names = BTreeSet::new();

    // When
    attribute_names(&selector("input[type=text]:not([disabled]) .x"), &mut names);
    attribute_names(&selector("#main .a"), &mut names);
    attribute_names(&selector(".b:is([hidden])"), &mut names);

    // Then
    let expected: BTreeSet<String> = ["type", "disabled", "id", "hidden"]
        .map(str::to_owned)
        .into_iter()
        .collect();
    assert_eq!(names, expected);
}

#[test]
fn a_cascade_knows_the_attribute_names_its_sheets_select() {
    // Given
    let cascade = Cascade::new()
        .user_agent(
            vapors_css::parse("[hidden] { display: none } #x { color: red }").expect("parses"),
        )
        .author(vapors_css::parse(".a { color: blue }").expect("parses"));

    // Then
    let names: Vec<&str> = cascade
        .selected_attributes()
        .iter()
        .map(String::as_str)
        .collect();
    assert_eq!(names, ["hidden", "id"]);
}

#[test]
fn the_rules_for_a_key_are_its_tag_class_universal_and_every_id_bucket() {
    // Given
    let sheet = ".row { } #x { } span { } * { } .other { } div .row { } div { }";
    let cascade = Cascade::new().author(vapors_css::parse(sheet).expect("parses"));

    // When
    let preludes: Vec<String> = cascade
        .rules_for_key("div", &["row", "marked"])
        .map(|rule| {
            vapors_css::serialize::component_values(&rule.prelude)
                .trim()
                .to_owned()
        })
        .collect();

    // Then
    assert_eq!(preludes, [".row", "#x", "*", "div .row", "div"]);
}
