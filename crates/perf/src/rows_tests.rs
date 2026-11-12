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
