use super::Children;
use crate::layout::{Reshaped, Shared};

/// `0..len` with `count` new slots (from 1000) put in at `at`, as the
/// layout hands the change over.
fn inserted(len: usize, at: usize, count: usize) -> (Vec<usize>, Reshaped) {
    let mut new: Vec<usize> = (0..len).collect();
    new.splice(at..at, 1000..1000 + count);
    let shared = Shared {
        head: at,
        tail: len - at,
    };
    (new, owner(shared, Vec::new()))
}

fn owner(shared: Shared, old_between: Vec<usize>) -> Reshaped {
    Reshaped {
        slot: 9999,
        shared,
        old_between,
    }
}

#[test]
fn a_child_is_kept_in_the_shared_head_and_tail_and_between_them_when_it_was_a_child() {
    // Given: two slots put in at 70 of 200, so 70..72 are new
    let (new, reshaped) = inserted(200, 70, 2);

    // When
    let children = Children::of(&new, &reshaped);

    // Then
    let not_kept: Vec<usize> = (0..new.len()).filter(|&at| !children.kept_at(at)).collect();
    assert_eq!(not_kept, vec![70, 71]);
    assert!(children.keeps_any());
}

#[test]
fn an_owner_whose_children_are_all_new_keeps_none() {
    // Given
    let (new, reshaped) = inserted(0, 0, 3);

    // When
    let children = Children::of(&new, &reshaped);

    // Then
    assert!(!children.keeps_any());
    assert!((0..3).all(|at| !children.kept_at(at)));
}

#[test]
fn an_owner_whose_children_were_all_replaced_keeps_none() {
    // Given: three children taken out, two new ones in
    let new = vec![1000, 1001];
    let reshaped = owner(Shared { head: 0, tail: 0 }, vec![1, 2, 3]);

    // When
    let children = Children::of(&new, &reshaped);

    // Then: the old children between the shared ends are not kept
    assert!(!children.keeps_any());
}
