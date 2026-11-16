//! A key's compiled rules (`src/cascade/compiled.rs`): the base
//! declarations enter without the matcher, at their rule's standing;
//! the tested rules and the rules of sheets the key does not reference
//! are matched on the element.

use crate::support;

use support::cascade::{BLUE, GREEN, RED};
use support::{chain, css};
use vapors_css::cascade::{Cascade, CompiledRules, Environment};
use vapors_css::tree::{States, Tree};

const OWN: &str = ".a { color: red; } .a:hover { color: blue; }";

fn cascade(sheets: &[&str]) -> Cascade {
    let parse = |text: &str| vapors_css::parse(text).expect("parses");
    sheets
        .iter()
        .fold(Cascade::new(), |cascade, text| cascade.author(parse(text)))
}

/// `.a`'s colour entered as base, `.a:hover` tested, sheet 0 referenced.
fn compiled() -> CompiledRules {
    CompiledRules {
        base: vec![(0, 0)],
        tested: vec![1],
        referenced: vec![0],
    }
}

fn colour(cascade: &Cascade, tree: &Tree, leaf: vapors_css::tree::NodeId) -> String {
    let style = cascade
        .compute_compiled(&tree.node(leaf), None, &Environment::default(), &compiled())
        .expect("computes");
    css(&style, "color")
}

#[test]
fn a_base_declaration_wins_as_the_full_cascade_says() {
    // Given
    let (tree, leaf) = chain(&["span.a"]);
    let cascade = cascade(&[OWN]);

    // Then
    let full = cascade
        .compute(&tree.node(leaf), None, &Environment::default())
        .expect("computes");
    assert_eq!(colour(&cascade, &tree, leaf), RED);
    assert_eq!(css(&full, "color"), RED);
}

#[test]
fn a_base_declaration_enters_without_the_matcher() {
    // Given: an element `.a` does not match; the key says it always does
    let (tree, leaf) = chain(&["span.b"]);

    // Then
    assert_eq!(colour(&cascade(&[OWN]), &tree, leaf), RED);
}

#[test]
fn a_tested_rule_that_matches_beats_the_base_by_its_standing() {
    // Given
    let (mut tree, leaf) = chain(&["span.a"]);
    tree.set_states(leaf, States::HOVER);

    // Then
    assert_eq!(colour(&cascade(&[OWN]), &tree, leaf), BLUE);
}

#[test]
fn a_rule_of_a_sheet_the_key_does_not_reference_is_matched_in_cascade_order() {
    // Given: another crate's sheet, added after, styling `.a` alike
    let (tree, leaf) = chain(&["span.a"]);
    let cascade = cascade(&[OWN, ".a { color: rgb(0, 128, 0); }"]);

    // Then: later and of equal specificity, it wins
    assert_eq!(colour(&cascade, &tree, leaf), GREEN);
    assert!(cascade.foreign_reaches("span", &["a"], &[0]));
    assert!(!cascade.foreign_reaches("span", &["a"], &[0, 1]));
}

#[test]
fn a_rule_is_found_by_its_sheet_and_its_place_in_the_sheet() {
    // Given
    let cascade = cascade(&[OWN, ".b { color: red; }"]);

    // Then
    assert_eq!(cascade.rule_index(0, 1), Some(1));
    assert_eq!(cascade.rule_index(1, 0), Some(2));
    assert_eq!(cascade.rule_index(1, 1), None);
}
