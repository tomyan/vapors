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
fn the_table_gives_the_slope_between_neighbouring_sizes_and_marks_the_baseline() {
    // When
    let table =
        memory_table(&[row(0, 0, 100), row(1000, 10, 2100), row(3000, 20, 6100)]).expect("table");

    // Then
    assert!(
        table.contains("| 0 | 1000 | 1000 | 1100 | 1105 | baseline |"),
        "{table}"
    );
    assert!(
        table.contains("| 1000 | 1000 | 1010 | 3100 | 3105 | 2.0 |"),
        "{table}"
    );
    assert!(
        table.contains("| 3000 | 1000 | 1020 | 7100 | 7105 | 2.0 |"),
        "{table}"
    );
}

#[test]
fn a_reading_that_fell_is_refused_naming_the_row() {
    let mut fell = row(1000, 10, 20);
    fell.after_frame_kib = fell.after_mount_kib - 1;
    let error = memory_table(&[fell]).expect_err("a fall");
    assert!(error.contains("1000 rows: a reading fell"), "{error}");
}

#[test]
fn rows_out_of_order_are_refused() {
    let error = memory_table(&[row(3000, 1, 2), row(1000, 1, 2)]).expect_err("order");
    assert!(error.contains("1000 rows follows 3000 rows"), "{error}");
}

#[test]
fn a_size_whose_frame_reading_is_below_the_one_before_shows_a_negative_slope() {
    // When
    let table = memory_table(&[row(1000, 10, 3000), row(3000, 10, 1000)]).expect("table");

    // Then
    assert!(
        table.contains("| 3000 | 1000 | 1010 | 2000 | 2005 | -1.0 |"),
        "{table}"
    );
}
