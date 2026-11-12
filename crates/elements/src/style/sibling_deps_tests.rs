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
fn a_nested_selector_of_one_compound_counts_and_one_with_combinators_reaches_its_own() {
    assert_eq!(deps(".row:not(:first-child)").next, 1);
    assert_eq!(
        deps(".row:is(.a + .b)"),
        SiblingDeps {
            next: 1,
            ..SiblingDeps::default()
        }
    );
}

#[test]
fn a_nested_selector_with_an_ancestor_group_reaches_subtrees_of_the_subject() {
    // `.cell:is(.a + .b .cell)`: the cells of the row after `.a`
    let own = deps(".cell:is(.a + .b .cell)");
    let before_subject = deps(".x + .y .cell:is(.a + .b)");

    assert_eq!((own.next, own.descendants), (1, true));
    assert_eq!((before_subject.next, before_subject.descendants), (1, true));
}
