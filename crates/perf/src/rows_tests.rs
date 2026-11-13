//! The Insert change: a row in the middle on one frame, gone the next.

use super::{RowChange, Rows};
use crate::run::Workload;

fn body_rows(rows: &Rows) -> usize {
    let doc = rows.page.doc().borrow();
    doc.children(doc.body()).len()
}

#[test]
fn insert_adds_a_row_before_the_middle_row_then_removes_it() {
    // Given
    let mut rows = Rows::new(10, RowChange::Insert);
    let middle = rows.page.rows().rows[5].element;

    // When
    rows.change(0);

    // Then: eleven rows, the new one just before the old middle row
    assert_eq!(body_rows(&rows), 11);
    let doc = rows.page.doc().borrow();
    let order = doc.children(doc.body());
    assert_eq!(order[6], middle);
    drop(doc);

    // When
    rows.change(1);

    // Then
    assert_eq!(body_rows(&rows), 10);
    rows.change(2);
    assert_eq!(body_rows(&rows), 11);
}

#[test]
fn the_other_changes_leave_the_row_count_alone() {
    for change in [
        RowChange::Idle,
        RowChange::Text,
        RowChange::FlexText,
        RowChange::Class,
    ] {
        let mut rows = Rows::new(10, change);
        rows.change(0);
        assert_eq!(body_rows(&rows), 10, "{change:?}");
    }
}

#[test]
fn changes_are_named_as_debug_prints_them() {
    for change in RowChange::ALL {
        assert!(
            matches!(RowChange::named(&format!("{change:?}")), Some(c) if format!("{c:?}") == format!("{change:?}"))
        );
    }
    assert!(RowChange::named("insert").is_none());
}

#[test]
fn insert_names_its_frames_in_the_order_it_makes_them() {
    // Given
    let mut rows = Rows::new(10, RowChange::Insert);
    let kinds = RowChange::Insert.frame_kinds();

    // When / Then: frame 0 inserts, frame 1 removes
    rows.change(0);
    assert_eq!((kinds[0], body_rows(&rows)), ("insert", 11));
    rows.change(1);
    assert_eq!((kinds[1], body_rows(&rows)), ("remove", 10));
    assert_eq!(kinds.len(), 2);
}

#[test]
fn a_change_whose_frames_are_alike_has_one_kind() {
    for change in [RowChange::Text, RowChange::Scroll] {
        assert_eq!(change.frame_kinds().len(), 1, "{change:?}");
    }
}

#[test]
fn resize_names_its_frames_in_the_order_it_makes_them() {
    // Given
    let mut rows = Rows::new(10, RowChange::Resize);
    let second = rows.page.rows().rows[1].element;
    let kinds = RowChange::Resize.frame_kinds();
    let class = |rows: &Rows| {
        rows.page
            .doc()
            .borrow()
            .attribute(second, "class")
            .map(str::to_owned)
    };

    // When / Then: frame 0 makes the second row tall, frame 1 short again
    rows.change(0);
    assert_eq!(
        (kinds[0], class(&rows).as_deref()),
        ("taller", Some("row tall"))
    );
    rows.change(1);
    assert_eq!(
        (kinds[1], class(&rows).as_deref()),
        ("shorter", Some("row"))
    );
    assert_eq!(kinds.len(), 2);
}

#[test]
fn scroll_size_names_its_frames_in_the_order_it_makes_them() {
    // Given
    let mut rows = Rows::new(100, RowChange::ScrollSize);
    let label = rows.page.rows().rows[50].label;
    let kinds = RowChange::ScrollSize.frame_kinds();
    let text = |rows: &Rows| rows.page.doc().borrow().text(label).map(str::to_owned);

    // When / Then
    rows.change(0);
    assert_eq!(
        (kinds[0], text(&rows).as_deref()),
        ("longer label", Some("A much longer label"))
    );
    rows.change(1);
    assert_eq!(
        (kinds[1], text(&rows).as_deref()),
        ("shorter label", Some("Label"))
    );
}
