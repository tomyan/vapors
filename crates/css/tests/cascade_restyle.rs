//! The incremental restyle rule: which elements a state-flag flip on one
//! element can restyle. vapors-specific.

mod support;

use support::{cascade, siblings};
use vapors_css::cascade::RestyleScope;
use vapors_css::selector::Element;
use vapors_css::tree::States;

#[test]
fn subject_pseudo_restyles_the_element_only() {
    assert_eq!(
        cascade(".a:hover { color: red; }").restyle_scope(States::HOVER),
        RestyleScope {
            element: true,
            ..RestyleScope::default()
        }
    );
}

#[test]
fn ancestor_pseudo_restyles_descendants() {
    assert_eq!(
        cascade(".a:hover > .b .c { color: red; }").restyle_scope(States::HOVER),
        RestyleScope {
            descendants: true,
            ..RestyleScope::default()
        }
    );
}

#[test]
fn sibling_pseudo_restyles_later_siblings_and_their_descendants() {
    assert_eq!(
        cascade(".a:focus + .b .c { color: red; }").restyle_scope(States::FOCUS),
        RestyleScope {
            later_siblings: true,
            ..RestyleScope::default()
        }
    );
}

#[test]
fn pseudo_inside_has_restyles_the_whole_tree() {
    assert_eq!(
        cascade("div:has(:checked) { color: red; }").restyle_scope(States::CHECKED),
        RestyleScope {
            whole_tree: true,
            ..RestyleScope::default()
        }
    );
}

#[test]
fn pseudo_inside_is_and_not_counts_at_its_compound() {
    assert_eq!(
        cascade(":is(.a:active, .b) span { color: red; } .c:not(:active) { color: red; }")
            .restyle_scope(States::ACTIVE),
        RestyleScope {
            element: true,
            descendants: true,
            ..RestyleScope::default()
        }
    );
}

#[test]
fn flags_nobody_selects_restyle_nothing() {
    assert_eq!(
        cascade(".a:hover { color: red; }").restyle_scope(States::FOCUS),
        RestyleScope::default()
    );
}

#[test]
fn restyle_set_enumerates_the_scope() {
    // Given: a + b(c) + d
    let (mut tree, kids) = siblings(&["div.a", "div.b", "div.d"]);
    let c = tree.append(kids[1], "span");
    let under_a = tree.append(kids[0], "span");
    let scope = RestyleScope {
        element: true,
        later_siblings: true,
        ..RestyleScope::default()
    };

    // When
    let set: Vec<_> = scope
        .elements(&tree.node(kids[0]))
        .into_iter()
        .map(vapors_css::tree::Node::id)
        .collect();

    // Then
    assert_eq!(set, [kids[0], kids[1], c, kids[2]]);
    assert!(!set.contains(&under_a));
    let all = RestyleScope {
        whole_tree: true,
        ..RestyleScope::default()
    }
    .elements(&tree.node(c));
    assert_eq!(all.len(), 6);
    assert!(all[0].parent().is_none());
}
