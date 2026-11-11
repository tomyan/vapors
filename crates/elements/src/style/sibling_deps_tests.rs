use super::SiblingDeps;

fn deps(selector: &str) -> SiblingDeps {
    let list = vapors_css::selector::parse_selector_list(selector).expect("parses");
    let mut deps = SiblingDeps::default();
    for selector in &list {
        deps.merge(SiblingDeps::of_selector(selector));
    }
    deps
}

fn all(descendants: bool) -> SiblingDeps {
    SiblingDeps {
        later: true,
        earlier: true,
        descendants,
        ..SiblingDeps::default()
    }
}

#[test]
fn plain_selectors_reach_no_sibling() {
    assert!(!deps(".row .cell, div > p:hover, :root").any());
}

#[test]
fn next_sibling_combinators_count_and_first_child_adds_one() {
    assert_eq!(deps(".a + .b + .c").next, 2);
    assert_eq!(deps(".a:first-child + .b").next, 2);
    assert_eq!(
        deps(":only-child"),
        SiblingDeps {
            next: 1,
            previous: true,
            ..SiblingDeps::default()
        }
    );
}

#[test]
fn a_group_before_the_subject_reaches_subtrees() {
    // Given
    let on_ancestor = deps(".a + .b .cell");
    let on_subject = deps(".row .a + .b");

    // Then
    assert_eq!((on_ancestor.next, on_ancestor.descendants), (1, true));
    assert_eq!((on_subject.next, on_subject.descendants), (1, false));
}

#[test]
fn uses_before_a_sibling_combinator_widen() {
    assert!(deps(":last-child ~ .b").later);
    assert_eq!(deps(":nth-last-child(2) + .b"), all(false));
}

#[test]
fn a_nested_selector_of_one_compound_counts_and_one_with_combinators_widens() {
    assert_eq!(deps(".row:not(:first-child)").next, 1);
    assert_eq!(deps(".row:is(.a + .b)"), all(false));
}
