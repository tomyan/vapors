//! The IU1 sparse-update workload: 100, 1000 and 3000 mounted rows,
//! each edit of `support::rows_page` followed by one update whose
//! structural counters are reported, and the result checked by the
//! correctness oracle (`support::oracle`) against a full recomputation. Counts only: timings belong to
//! the baseline in docs/plans/iu1.md. `cargo nextest run -p
//! vapors-elements --test sparse_updates --no-capture` prints the table.

mod support;

use support::oracle::assert_matches_full;
use support::rows_page::{EDITS, Edit, RowsPage, rows_page, rows_viewport};
use vapors_elements::Document;

/// What one update after an edit did, as structural counts.
#[derive(Debug)]
struct Work {
    changed: bool,
    restyled: usize,
    laid_out: usize,
    text_layouts: usize,
}

impl Work {
    fn of(doc: &mut Document) -> Self {
        let changed = doc.take_changed();
        doc.update(rows_viewport())
            .unwrap_or_else(|errors| panic!("refused: {errors:#?}"));
        Self {
            changed,
            restyled: doc.elements_restyled(),
            laid_out: doc.boxes_laid_out(),
            text_layouts: doc.text_layouts_built(),
        }
    }
}

/// When the oracle runs: a full update of 3000 rows takes about 5 s in
/// a debug build on the dev machine (2026-11-10), so the largest page
/// is checked once, after the whole sequence, within the test deadline.
#[derive(Clone, Copy, PartialEq)]
enum Check {
    EachEdit,
    AfterSequence,
}

fn run(rows: usize, check: Check) {
    let page = rows_page(rows);
    for edit in EDITS {
        let work = edited(&page, edit);
        eprintln!("{rows:>5} rows  {edit:<16?} {work:?}");
        assert_sparse(rows, edit, &work);
        if check == Check::EachEdit {
            assert_matches_full(
                &page.doc,
                rows_viewport(),
                &format!("{rows} rows, {edit:?}"),
            );
        }
    }
    if check == Check::AfterSequence {
        assert_matches_full(
            &page.doc,
            rows_viewport(),
            &format!("{rows} rows, {EDITS:?}"),
        );
    }
}

fn edited(page: &RowsPage, edit: Edit) -> Work {
    page.doc.borrow_mut().take_changed();
    page.edit(edit);
    Work::of(&mut page.doc.borrow_mut())
}

/// The cache-miss counters stay small whatever the row count: what is
/// sparse today. The traversal counters are reported, not held.
fn assert_sparse(rows: usize, edit: Edit, work: &Work) {
    assert!(
        work.changed,
        "{rows} rows, {edit:?}: the edit noted no change"
    );
    assert!(
        work.restyled <= MAX_RESTYLED
            && work.laid_out <= MAX_LAID_OUT
            && work.text_layouts <= MAX_TEXT_LAYOUTS,
        "{rows} rows, {edit:?}: {work:?} grew past the sparse bounds; reproduce: cargo nextest \
         run -p vapors-elements --test sparse_updates --no-capture"
    );
}

/// A row's subtree and the button's ancestors.
const MAX_RESTYLED: usize = 4;
/// The label measured at each width Taffy asks for.
const MAX_TEXT_LAYOUTS: usize = 3;
/// A row and its ancestors, and their siblings Taffy lays out again.
const MAX_LAID_OUT: usize = 12;

#[test]
fn a_hundred_rows_update_sparsely_and_match_a_full_update() {
    run(100, Check::EachEdit);
}

#[test]
fn a_thousand_rows_update_sparsely_and_match_a_full_update() {
    run(1000, Check::AfterSequence);
}

#[test]
fn three_thousand_rows_update_sparsely_and_match_a_full_update() {
    run(3000, Check::AfterSequence);
}
