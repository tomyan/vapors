use super::{reserve, resize, resize_with};

/// A list of `len` entries in exactly `len` slots, as a first build leaves it.
fn exact(len: usize) -> Vec<u8> {
    vec![0; len]
}

#[test]
fn one_entry_more_grows_a_full_list_by_a_quarter() {
    // Given
    let mut list = exact(400);

    // When
    resize(&mut list, 401, 1);

    // Then
    assert_eq!((list.len(), list.capacity()), (401, 500));
}

#[test]
fn many_entries_more_grow_it_to_the_count_asked() {
    // Given
    let mut list = exact(400);

    // When
    resize_with(&mut list, 900, || 1);

    // Then
    assert_eq!((list.len(), list.capacity()), (900, 900));
}

#[test]
fn a_list_with_room_or_shrinking_keeps_its_slots() {
    // Given
    let mut list = exact(400);
    reserve(&mut list, 401);

    // When
    resize(&mut list, 450, 1);
    resize(&mut list, 10, 1);

    // Then
    assert_eq!((list.len(), list.capacity()), (10, 500));
}
