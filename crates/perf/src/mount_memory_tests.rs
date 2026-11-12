//! The memory rows: the line a one-size process prints and the parent
//! reads back, and the table of them.

use super::{MemoryRow, memory_table, parse_line};

fn row(rows: usize, mounted: u64, framed: u64) -> MemoryRow {
    MemoryRow {
        rows,
        before_kib: 1000,
        after_mount_kib: 1000 + mounted,
        after_frame_kib: 1000 + framed,
        peak_kib: 1000 + framed + 5,
    }
}

#[test]
fn a_row_survives_its_line() {
    // Given
    let sent = row(3000, 20, 300);

    // When
    let read = parse_line(&sent.line()).expect("parsed");

    // Then
    assert_eq!(read, sent);
}

#[test]
fn a_line_with_a_missing_or_extra_field_is_refused_naming_it() {
    for line in [
        "mount-memory rows=1 before=2",
        "mount-memory rows=1 before=2 after_mount=3 after_frame=4 peak=5 x=6",
        "other rows=1",
    ] {
        let error = parse_line(line).expect_err(line);
        assert!(error.contains(line), "{error}");
    }
}

#[test]
fn a_field_that_is_not_a_count_is_refused_naming_the_line() {
    let line = "mount-memory rows=1 before=2 after_mount=3 after_frame=x peak=5";
    let error = parse_line(line).expect_err("not a count");
    assert!(error.contains(line), "{error}");
}

#[test]
fn the_table_gives_the_growth_per_row_at_the_mount_and_after_the_frame() {
    // When
    let table = memory_table(&[row(1000, 10, 2000)]);

    // Then
    assert!(
        table.contains("| 1000 | 1000 | 1010 | 3000 | 3005 | 10 | 2048 |"),
        "{table}"
    );
}
