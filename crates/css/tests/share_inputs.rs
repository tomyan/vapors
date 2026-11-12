//! Whether a rule blocks sharing one computed style between the
//! elements it can match (analysis PR 2): a selector that reads
//! siblings or descendants, or an `@container` condition, which reads
//! layout. `@media` and `@supports` are frame-wide and do not.

use vapors_css::cascade::{Unshareable, unshareable};

/// Why the first rule of `sheet` blocks sharing.
fn reason(sheet: &str) -> Option<Unshareable> {
    let parsed = vapors_css::parse(sheet).unwrap_or_else(|e| panic!("{sheet}: {e}"));
    unshareable(&parsed.rules[0])
}

#[test]
fn ancestor_combinators_states_and_frame_wide_conditions_do_not_block_sharing() {
    for sheet in [
        ".a .b {}",
        ".a > .b {}",
        "div.a:hover > .b:focus-within {}",
        ".a:is(.b, .c) {}",
        ".a:not(.b) {}",
        ".a:where([x]) {}",
        ":global(.g) .a {}",
        "input[type=text]:checked:disabled {}",
        ":root .a {}",
        "@media (min-width: 100px) { .a { color: red } }",
        "@supports (display: grid) { .a { color: red } }",
    ] {
        assert_eq!(reason(sheet), None, "{sheet}");
    }
}

#[test]
fn every_sibling_or_descendant_read_blocks_sharing_wherever_it_sits() {
    for sheet in [
        ".a + .b {}",
        ".a ~ .b .c {}",
        ".a:first-child {}",
        ".a:last-child > .b {}",
        ".a:only-child {}",
        "li:first-of-type {}",
        "li:last-of-type {}",
        "li:only-of-type {}",
        "li:nth-child(2) {}",
        "li:nth-last-child(2) {}",
        "li:nth-of-type(2) {}",
        "li:nth-last-of-type(2) {}",
        ".a:empty {}",
        ".a:has(.b) {}",
        ".a:has(> .b) {}",
        ".a:not(.b + .c) {}",
        ".a:is(.b:only-child) {}",
        ".a:where(.b ~ .c) {}",
        ":global(.g:last-child) .a {}",
        ":is(.p:last-of-type) > .q {}",
    ] {
        assert_eq!(
            reason(sheet),
            Some(Unshareable::ReadsSiblingsOrDescendants),
            "{sheet}"
        );
    }
}

#[test]
fn a_rule_under_container_blocks_sharing_however_deep() {
    for sheet in [
        "@container (min-width: 100px) { .a { color: red } }",
        "@media (min-width: 1px) { @container (min-width: 100px) { .a { color: red } } }",
    ] {
        assert_eq!(reason(sheet), Some(Unshareable::InContainer), "{sheet}");
    }
}

#[test]
fn a_container_rule_a_key_can_match_is_reported_by_the_cascade() {
    // Given: the review's probe, a key's rule under @container
    let sheet = "@container (min-width: 100px) { .a { color: red } }";
    let cascade =
        vapors_css::cascade::Cascade::new().author(vapors_css::parse(sheet).expect("parses"));

    // When
    let reasons: Vec<_> = cascade
        .rules_for_key("div", &["a"])
        .map(unshareable)
        .collect();

    // Then
    assert_eq!(reasons, [Some(Unshareable::InContainer)]);
}
