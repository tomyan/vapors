//! The Mount workload's rows share computed styles (analysis PR 2):
//! the first update computes one style per distinct share key, however
//! many rows, and keeps no more styles alive at 3,000 rows than at 300.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_elements::{Document, UpdateCounts, Viewport};
use vapors_signals::Signal;
use vapors_text::FontSystem;

use super::row_data;
use crate::mount_rows::{MountRows, MountRowsProps};

/// The counts of the first update of `count` compiled rows.
fn first_update(count: usize) -> UpdateCounts {
    let doc = Rc::new(RefCell::new(Document::new(FontSystem::bundle_only())));
    let body = doc.borrow().body();
    let rows = Signal::new(row_data(count));
    let _rows = MountRows::mount(&doc, body, MountRowsProps { rows });
    let viewport = Viewport {
        width: 640.0,
        height: 400.0,
        scale: 1.0,
    };
    doc.borrow_mut().update(viewport).expect("updates");
    doc.borrow().update_counts()
}

#[test]
fn rows_compute_one_style_and_one_used_style_per_share_key_at_any_count() {
    // When
    let (small, large) = (first_update(300), first_update(3000));

    // Then: html and body are unkeyed; the row, its label, value and
    // button compute one style and derive one used style each, which
    // every other row shares
    for (rows, counts) in [(300, small), (3000, large)] {
        assert_eq!(counts.elements_restyled, rows * 4 + 2, "{rows} rows");
        assert_eq!(counts.styles_unkeyed, 2, "{rows} rows");
        assert_eq!(counts.styles_computed, 4, "{rows} rows: {counts:?}");
        assert_eq!(counts.styles_shared, rows * 4 - 4, "{rows} rows");
        assert_eq!(counts.styles_live, 6, "{rows} rows: {counts:?}");
        assert_eq!(counts.used_derived, 6, "{rows} rows: {counts:?}");
        assert_eq!(counts.used_shared, rows * 4 - 4, "{rows} rows");
    }
}

#[test]
fn every_compiled_row_element_has_winners_whose_sheets_hash_as_registered() {
    // Given
    let doc = Rc::new(RefCell::new(Document::new(FontSystem::bundle_only())));
    let body = doc.borrow().body();
    let _rows = MountRows::mount(
        &doc,
        body,
        MountRowsProps {
            rows: Signal::new(row_data(2)),
        },
    );
    let doc = doc.borrow();

    // When: every element under the body
    let mut pending = doc.children(body);
    let mut checked = 0;
    while let Some(node) = pending.pop() {
        pending.extend(doc.children(node));
        let Some(key) = vapors_dom::Dom::style_key(&*doc, node) else {
            continue;
        };

        // Then
        assert!(
            key.winners.is_some(),
            "{} has no compiled winners",
            key.site
        );
        doc.check_compiled_sheets(key)
            .unwrap_or_else(|refusal| panic!("{refusal}"));
        checked += 1;
    }
    assert_eq!(checked, 8, "two rows of a div, two spans and a button");
}
