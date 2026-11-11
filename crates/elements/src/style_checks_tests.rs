//! `walked` when the kept checks and a walk over the tree disagree.

use std::collections::HashSet;

use vapors_dom::NodeId;

use super::walked;
use crate::error::ElementError;

/// Kept checks saying `<div>` (node 7) refuses.
fn kept() -> HashSet<NodeId> {
    HashSet::from([NodeId(7)])
}

fn describe(node: NodeId) -> String {
    format!("<div> (node {})", node.0)
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "a change was not marked")]
fn a_debug_build_panics_when_a_kept_refusal_finds_none_in_the_tree() {
    walked(&kept(), Vec::new, "tree", describe);
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "a change was not marked")]
fn a_debug_build_panics_when_the_tree_refuses_and_the_kept_checks_do_not() {
    let found = || vec![ElementError::UnsupportedTag("<canvas>".to_owned())];
    walked(&HashSet::new(), found, "tree", describe);
}

#[test]
#[cfg(not(debug_assertions))]
fn a_release_build_refuses_naming_the_element_when_a_kept_refusal_finds_none() {
    // When
    let errors = walked(&kept(), Vec::new, "tree", describe);

    // Then
    let [ElementError::Unmarked(message)] = errors.as_slice() else {
        panic!("one Unmarked refusal expected: {errors:?}");
    };
    assert!(
        message.starts_with("<div> (node 7): the kept tree checks"),
        "{message}"
    );
    assert!(message.contains("reproduce in a debug build"), "{message}");
}
