use super::Slab;

#[test]
fn a_freed_id_is_taken_by_the_next_insert() {
    // Given
    let mut slab = Slab::default();
    let (first, second) = (slab.insert("a"), slab.insert("b"));

    // When
    slab.remove(first);
    let third = slab.insert("c");

    // Then
    assert_eq!((first, second, third), (0, 1, 0));
    assert_eq!(slab.get(third), Some(&"c"));
    assert_eq!(slab.live(), 2);
}

#[test]
fn a_freed_id_resolves_to_nothing() {
    // Given
    let mut slab = Slab::default();
    let id = slab.insert("a");

    // When
    slab.remove(id);

    // Then
    assert_eq!(slab.get(id), None);
    assert_eq!(slab.live_ids().count(), 0);
}

#[test]
#[should_panic(expected = "slab id 0 freed while not held")]
fn freeing_an_id_twice_is_refused() {
    // Given
    let mut slab = Slab::default();
    let id = slab.insert("a");
    slab.remove(id);

    // When, Then
    slab.remove(id);
}
