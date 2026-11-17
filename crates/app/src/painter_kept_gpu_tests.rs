use super::freed_entry;

#[test]
fn a_kept_batch_naming_a_freed_entry_in_any_run_is_found() {
    // Given: runs naming entries 3, 0 (none) and 9
    let clips = [(0, 3), (2, 0), (5, 9)];

    // Then
    assert_eq!(freed_entry(&clips, &[9, 12]), Some(9));
    assert_eq!(freed_entry(&clips, &[4]), None);
}
