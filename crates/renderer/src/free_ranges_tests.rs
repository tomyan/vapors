use super::*;

#[test]
fn takes_from_the_first_free_range_that_holds_the_bytes() {
    // Given: 0..16 and 48..112 free
    let mut free = FreeRanges::new(112);
    let (a, b) = (free.take(16), free.take(32));
    free.give(a.expect("room")..16);

    // When
    let taken = free.take(32);

    // Then: 0..16 is too small; 48..80 is first
    assert_eq!((a, b, taken), (Some(0), Some(16), Some(48)));
    assert_eq!(free.free_bytes(), 16 + 32);
}

#[test]
fn refuses_bytes_no_free_range_holds() {
    // Given: 32 bytes free, in two ranges of 16
    let mut free = FreeRanges::new(64);
    let _ = free.take(64);
    free.give(0..16);
    free.give(32..48);

    // When
    let taken = free.take(32);

    // Then
    assert_eq!(taken, None);
    assert_eq!(free.free_bytes(), 32);
}

#[test]
fn a_range_given_back_joins_the_free_ranges_either_side() {
    // Given: 0..16 and 32..64 free, 16..32 held
    let mut free = FreeRanges::new(64);
    let _ = free.take(64);
    free.give(0..16);
    free.give(32..64);

    // When
    free.give(16..32);

    // Then: one range, so the whole buffer can be taken at once
    assert_eq!(free, FreeRanges::new(64));
    assert_eq!(free.take(64), Some(0));
}

#[test]
fn a_range_given_back_joins_the_one_before_or_after_it() {
    for (given, then, whole) in [(0..16, 16..32, 0..32), (16..32, 0..16, 0..32)] {
        // Given
        let mut free = FreeRanges::new(64);
        let _ = free.take(64);
        free.give(given.clone());

        // When
        free.give(then.clone());

        // Then
        let mut expected = FreeRanges::new(64);
        let _ = expected.take(64);
        expected.give(whole);
        assert_eq!(free, expected, "{given:?} then {then:?}");
    }
}

#[test]
fn after_holds_the_first_bytes() {
    // Given
    let mut free = FreeRanges::after(48, 64);

    // When
    let taken = free.take(16);

    // Then
    assert_eq!(taken, Some(48));
    assert_eq!(free.free_bytes(), 0);
}

#[test]
#[should_panic(expected = "freed twice, or never taken")]
fn a_range_given_back_twice_panics() {
    // Given
    let mut free = FreeRanges::new(64);
    let _ = free.take(32);
    free.give(0..16);

    // When
    free.give(0..16);
}

#[test]
#[should_panic(expected = "freed twice, or never taken")]
fn a_range_past_the_buffer_panics() {
    // Given
    let mut free = FreeRanges::new(64);

    // When
    free.give(64..80);
}

#[test]
#[should_panic(expected = "freed twice, or never taken")]
fn a_range_overlapping_the_free_range_before_it_panics() {
    // Given: 0..32 free
    let mut free = FreeRanges::new(64);
    let _ = free.take(64);
    free.give(0..32);

    // When
    free.give(16..48);
}
