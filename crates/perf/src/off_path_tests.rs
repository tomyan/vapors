//! An off-path workload makes its change before each frame, through
//! the app's document, with no GPU.

use super::{Kind, OffPath, OffPathChange};
use crate::run::Workload;

#[test]
fn every_kind_and_change_makes_three_changes() {
    for kind in Kind::ALL {
        for &change in kind.changes() {
            let mut workload = OffPath::new(kind, 200, change);
            for sample in 0..3 {
                workload.change(sample);
            }
        }
    }
}

#[test]
fn an_edit_reaches_the_documents_middle_unit() {
    // Given
    let mut workload = OffPath::new(Kind::NestedScrollers, 60, OffPathChange::Edit);
    let value = workload.page.units[30].value;

    // When
    workload.change(7);

    // Then
    assert_eq!(workload.page.doc.borrow().text(value), Some("90007"));
}
