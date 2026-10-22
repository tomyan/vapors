//! The incremental restyle rule (`src/cascade/restyle.rs`). vapors-specific.

mod support;

use support::{cascade, siblings};
use vapors_css::cascade::RestyleScope;
use vapors_css::selector::Element;
use vapors_css::tree::States;

#[test]
fn subject_pseudo_restyles_the_element_only() {
    assert_eq!(
        cascade(".a:hover { opacity: 0.5; }").restyle_scope(States::HOVER),
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
        cascade(":is(.a:active, .b) span { color: red; } .c:not(:active) { opacity: 0.5; }")
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
        cascade(".a:hover { opacity: 0.5; }").restyle_scope(States::FOCUS),
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

#[test]
fn only_the_combinator_right_after_the_flagged_compound_picks_the_scope() {
    // Given: `.a:hover .b + .c`: .c is a sibling of .b, which is inside .a,
    // so .c is in .a's subtree, not among .a's siblings.
    assert_eq!(
        cascade(".a:hover .b + .c { color: red; }").restyle_scope(States::HOVER),
        RestyleScope {
            descendants: true,
            ..RestyleScope::default()
        }
    );
    assert_eq!(
        cascade(".a:hover > .b ~ .c .d { color: red; }").restyle_scope(States::HOVER),
        RestyleScope {
            descendants: true,
            ..RestyleScope::default()
        }
    );
    // `.a:hover + .b .c`: .c is inside a later sibling of .a.
    assert_eq!(
        cascade(".a:hover + .b .c { color: red; }").restyle_scope(States::HOVER),
        RestyleScope {
            later_siblings: true,
            ..RestyleScope::default()
        }
    );
    assert_eq!(
        cascade(".a:hover ~ .b > .c + .d { color: red; }").restyle_scope(States::HOVER),
        RestyleScope {
            later_siblings: true,
            ..RestyleScope::default()
        }
    );
}

#[test]
fn subject_rules_that_set_inherited_properties_restyle_the_subtree() {
    // Given: colour inherits, so the element's descendants change too.
    assert_eq!(
        cascade(".a:hover { color: red; }").restyle_scope(States::HOVER),
        RestyleScope {
            element: true,
            descendants: true,
            ..RestyleScope::default()
        }
    );
    // font-size drives em on descendants; custom properties inherit.
    assert_eq!(
        cascade(".a:focus { font-size: 2em; }").restyle_scope(States::FOCUS),
        RestyleScope {
            element: true,
            descendants: true,
            ..RestyleScope::default()
        }
    );
    assert_eq!(
        cascade(".a:focus { --x: 1; }").restyle_scope(States::FOCUS),
        RestyleScope {
            element: true,
            descendants: true,
            ..RestyleScope::default()
        }
    );
    // A non-inherited property stays on the element.
    assert_eq!(
        cascade(".a:active { opacity: 0.5; width: 1px; }").restyle_scope(States::ACTIVE),
        RestyleScope {
            element: true,
            ..RestyleScope::default()
        }
    );
}
