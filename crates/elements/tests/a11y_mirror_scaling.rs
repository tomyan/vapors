//! A take's work follows what was touched, never the page (R6, review
//! 230-M2 and 230-M3; `docs/plans/r6.md` "Update strategy"): held by
//! `AxTakeCounts` across page sizes, as `UpdateCounts` holds an
//! update's. A focus move and a state change read the same nodes on a
//! page of 100 rows and of 400, patch the index for the nodes touched
//! and scan nothing; a scroll touches the rows it moved (the cost the
//! plan names) and scans nothing; a role removed over K children orders
//! its K placements in steps that grow with K, not K squared.

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use support::roles_page::button;
use support::rows_page::{RowsPage, rows_page, rows_viewport};
use support::{document, element, viewport};
use vapors_dom::{Dom, NodeId};
use vapors_elements::{AxTakeCounts, Document};

/// `count` rows, each button carrying an id, with the first take taken.
fn taken_rows(count: usize) -> RowsPage {
    let page = rows_page(count);
    {
        let mut doc = page.doc.borrow_mut();
        for (index, row) in page.rows.iter().enumerate() {
            doc.set_attribute(row.button, "id", &format!("b{index}"));
        }
        doc.add_stylesheet(
            "scroll.css",
            "html { overflow: hidden; } body { height: 400px; overflow: auto; }",
        )
        .expect("parses");
        doc.update(rows_viewport()).expect("updates");
        doc.take_ax_update().expect("updated").expect("every");
    }
    page
}

/// Updates and takes; the take's counts.
fn counts_of(doc: &Rc<RefCell<Document>>) -> AxTakeCounts {
    let mut doc = doc.borrow_mut();
    doc.update(rows_viewport()).expect("updates");
    doc.take_ax_update().expect("updated").expect("a change");
    doc.ax_take_counts()
}

fn focus_move(page: &RowsPage) -> AxTakeCounts {
    Document::focus(&page.doc, Some(page.rows[3].button));
    counts_of(&page.doc)
}

fn state_change(page: &RowsPage) -> AxTakeCounts {
    page.doc
        .borrow_mut()
        .set_attribute(page.rows[5].button, "aria-pressed", "true");
    counts_of(&page.doc)
}

fn scroll(page: &RowsPage) -> AxTakeCounts {
    assert!(page.doc.borrow_mut().scroll_by([10.0, 10.0], [0.0, 48.0]));
    counts_of(&page.doc)
}

#[test]
fn a_focus_move_reads_the_same_nodes_on_a_page_four_times_the_size() {
    // Given
    let (small, large) = (taken_rows(100), taken_rows(400));

    // When
    let (a, b) = (focus_move(&small), focus_move(&large));

    // Then
    assert_eq!(a, b, "the work of a focus move");
    assert_eq!(a.index_scans, 0);
    assert!(a.nodes_read <= 4, "{a:?}");
}

#[test]
fn a_state_change_reads_the_same_nodes_on_a_page_four_times_the_size() {
    // Given
    let (small, large) = (taken_rows(100), taken_rows(400));

    // When
    let (a, b) = (state_change(&small), state_change(&large));

    // Then
    assert_eq!(a, b, "the work of a state change");
    assert_eq!(a.index_scans, 0);
    assert!(a.nodes_read <= 4, "{a:?}");
}

#[test]
fn a_scroll_touches_the_rows_it_moved_and_scans_nothing() {
    // Given
    let (small, large) = (taken_rows(100), taken_rows(400));

    // When
    let (a, b) = (scroll(&small), scroll(&large));

    // Then: every row's four elements moved, the cost the plan names,
    // each read with body and html above; the index patched for them,
    // never scanned
    assert_eq!((a.nodes_read, b.nodes_read), (4 * 100 + 2, 4 * 400 + 2));
    assert_eq!((a.index_entries, b.index_entries), (4 * 100, 4 * 400));
    assert_eq!((a.index_scans, b.index_scans), (0, 0));
}

/// A group of `count` buttons, taken, with its role then removed.
fn group_losing_its_role(count: usize) -> (Document, NodeId) {
    let mut doc = document();
    let body = doc.body();
    let group = element(&mut doc, body, "div", "");
    doc.set_attribute(group, "role", "group");
    doc.set_attribute(group, "aria-label", "G");
    for index in 0..count {
        button(&mut doc, group, &format!("b{index}"), &[]);
    }
    doc.add_stylesheet("s.css", "button { display: block; height: 4px; }")
        .expect("parses");
    doc.update(viewport(640.0, 400.0)).expect("updates");
    doc.take_ax_update().expect("updated").expect("every");
    doc.set_attribute(group, "role", "none");
    (doc, group)
}

#[test]
fn a_role_removed_over_k_children_orders_k_placements_in_one_pass() {
    // Given
    let (mut small, _) = group_losing_its_role(200);
    let (mut large, _) = group_losing_its_role(800);

    // When
    let counts = |doc: &mut Document| {
        doc.update(viewport(640.0, 400.0)).expect("updates");
        let update = doc.take_ax_update().expect("updated").expect("placed");
        (
            update.placed.len(),
            doc.ax_take_counts().placements_considered,
        )
    };
    let (a, b) = (counts(&mut small), counts(&mut large));

    // Then: 800 placements in at most 4.5 times the steps of 200
    assert_eq!((a.0, b.0), (200, 800));
    assert!(b.1 * 2 <= a.1 * 9, "{a:?} then {b:?}");
    assert!(a.1 <= 3 * 200, "{a:?}");
}
