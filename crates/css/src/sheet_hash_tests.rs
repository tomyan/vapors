use super::{hash_with, sheet_hash};

#[test]
fn a_sheet_hashes_the_same_each_time() {
    assert_eq!(
        sheet_hash(".row { color: red }"),
        sheet_hash(".row { color: red }")
    );
}

#[test]
fn any_change_to_a_sheets_text_changes_its_hash() {
    // Given
    let text = ".row { color: red }";

    // Then
    for other in [
        ".row { color: blue }",
        ".row { color: red } ",
        ".rows { color: red }",
        "",
    ] {
        assert_ne!(sheet_hash(text), sheet_hash(other), "{other:?}");
    }
}

#[test]
fn another_version_of_vapors_css_hashes_the_same_text_differently() {
    assert_ne!(hash_with("0.0.0", "a {}"), hash_with("0.0.1", "a {}"));
    assert_ne!(hash_with("0.0.0", "1a {}"), hash_with("0.0.01", "a {}"));
}

#[test]
fn the_hash_is_fnv_1a_as_published() {
    // FNV-1a 64 of the empty input is its offset basis, and of "a" the
    // published 0xaf63dc4c8601ec8c; the version and separator prefix it
    assert_eq!(super::OFFSET, 0xcbf2_9ce4_8422_2325);
    let a = (super::OFFSET ^ u64::from(b'a')).wrapping_mul(super::PRIME);
    assert_eq!(a, 0xaf63_dc4c_8601_ec8c);
}
