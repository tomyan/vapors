//! The edit's changed range and its UTF-16 ranges.

use super::*;

#[test]
fn the_changed_range_lies_between_the_common_start_and_end() {
    for (before, after, range) in [
        ("Warm", "Warmth", 4..4),
        ("Warm", "War", 3..4),
        ("aa", "aaa", 2..2),
        ("a😀b", "a😀cb", 5..5),
        ("a😀b", "ab", 1..5),
        ("Warm", "Cool", 0..4),
        ("same", "same", 4..4),
    ] {
        assert_eq!(changed(before, after), range, "{before:?} to {after:?}");
    }
}

#[test]
fn utf16_ranges_become_bytes_and_a_split_character_is_refused() {
    assert_eq!(bytes("a😀b", (1, 3), "selection"), Ok(1..5));
    assert_eq!(bytes("a😀b", (4, 4), "selection"), Ok(6..6));
    assert!(bytes("a😀b", (2, 2), "selection").is_err());
    assert!(bytes("ab", (2, 3), "selection").is_err());
    assert!(bytes("ab", (2, 1), "selection").is_err());
}
