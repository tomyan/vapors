use super::{Entries, IDENTITY, translation};

#[test]
fn entry_0_is_the_identity_and_never_handed_out() {
    // Given
    let mut entries = Entries::new(4);

    // When
    let taken = [entries.take(1), entries.take(2), entries.take(1)];

    // Then
    assert_eq!(taken, [Some(1), Some(2), None]);
    assert_eq!(entries.bytes((0, 0)).len(), 64);
}

#[test]
fn a_set_reports_only_the_span_that_changed() {
    // Given
    let mut entries = Entries::new(8);
    let at = entries.take(3).expect("room");

    // When: the middle of three changes
    let changed = entries.set(at, &[IDENTITY, translation([0.0, 24.0]), IDENTITY], 1);

    // Then
    assert_eq!(changed, Some((at + 1, at + 1)));
    assert_eq!(
        entries.set(at, &[IDENTITY, translation([0.0, 24.0]), IDENTITY], 2),
        None
    );
}

#[test]
#[should_panic(expected = "written twice in one frame")]
fn an_entry_changed_twice_for_one_encoder_is_refused() {
    let mut entries = Entries::new(4);
    let at = entries.take(1).expect("room");
    entries.set(at, &[translation([1.0, 0.0])], 7);
    entries.set(at, &[translation([2.0, 0.0])], 7);
}

#[test]
fn growing_frees_the_new_entries() {
    // Given: full
    let mut entries = Entries::new(2);
    assert_eq!(entries.take(1), Some(1));
    assert_eq!(entries.take(1), None);

    // When
    entries.grow(4);

    // Then
    assert_eq!((entries.capacity(), entries.take(3)), (4, None));
    assert_eq!(entries.take(2), Some(2));
}
