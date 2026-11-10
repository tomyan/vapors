//! The IU1 sparse-update workload: 100 and 1000 mounted rows (and any
//! count in `VAPORS_SPARSE_ROWS`, 3000 for the baseline), each edit of
//! `support::rows_page` followed by one update whose structural
//! counters are reported, and the result checked by the correctness
//! oracle (`support::oracle`) against a full recomputation. Counts
//! only: timings belong to the baseline in docs/plans/iu1.md.
//! `cargo nextest run -p vapors-elements --test sparse_updates
//! --no-capture` prints the table.

mod support;

use support::oracle::assert_matches_full;
use support::rows_page::{EDITS, Edit, RowsPage, rows_page, rows_viewport};
use vapors_elements::Document;

/// What one update after an edit did, as structural counts.
#[derive(Debug)]
struct Work {
    restyled: usize,
    laid_out: usize,
    text_layouts: usize,
}

impl Work {
    fn of(doc: &mut Document) -> Self {
        doc.update(rows_viewport())
            .unwrap_or_else(|errors| panic!("refused: {errors:#?}"));
        Self {
            restyled: doc.elements_restyled(),
            laid_out: doc.boxes_laid_out(),
            text_layouts: doc.text_layouts_built(),
        }
    }
}

/// When the oracle runs. A full update of 3000 rows took 23.3 s alone
/// and 29.2 s at -j4 on the dev machine (2026-11-10, debug build);
/// `.config/nextest.toml` marks a test slow at 30 s and kills it at
/// 60 s, so 3000 rows run only when `VAPORS_SPARSE_ROWS=3000` asks, and
/// are checked once after the whole sequence.
#[derive(Clone, Copy, PartialEq)]
enum Check {
    EachEdit,
    AfterSequence,
}

/// The counts after each edit of `EDITS`, in order, at every row count:
/// restyled, laid out, text layouts built. The table in
/// docs/plans/iu1.md is these numbers; a change here updates it.
const EXPECTED: [(usize, usize, usize); 5] =
    [(0, 5, 1), (0, 9, 3), (0, 0, 0), (4, 0, 0), (1, 0, 0)];

fn run(rows: usize, check: Check) {
    let page = rows_page(rows);
    for (edit, expected) in EDITS.into_iter().zip(EXPECTED) {
        let work = edited(&page, edit);
        eprintln!("{rows:>5} rows  {edit:<16?} {work:?}");
        assert_counts(rows, edit, &work, expected);
        if check == Check::EachEdit {
            assert_oracle(&page, rows, &format!("{edit:?}"));
        }
    }
    if check == Check::AfterSequence {
        assert_oracle(&page, rows, &format!("{EDITS:?}"));
    }
}

fn assert_oracle(page: &RowsPage, rows: usize, edits: &str) {
    assert_matches_full(&page.doc, rows_viewport(), &format!("{rows} rows, {edits}"));
}

fn edited(page: &RowsPage, edit: Edit) -> Work {
    page.doc.borrow_mut().take_changed();
    page.edit(edit);
    Work::of(&mut page.doc.borrow_mut())
}

/// The cache-miss counters are exactly the table's, whatever the row
/// count. The traversal counters are reported, not held. Whether an
/// edit reports a change is not asserted: an equal write is reported as
/// one today, and IU's direction will change that.
fn assert_counts(rows: usize, edit: Edit, work: &Work, expected: (usize, usize, usize)) {
    let actual = (work.restyled, work.laid_out, work.text_layouts);
    assert_eq!(
        actual, expected,
        "{rows} rows, {edit:?}: {work:?}; update EXPECTED and docs/plans/iu1.md; reproduce: \
         cargo nextest run -p vapors-elements --test sparse_updates --no-capture"
    );
}

#[test]
fn a_hundred_rows_update_sparsely_and_match_a_full_update() {
    run(100, Check::EachEdit);
}

#[test]
fn a_thousand_rows_update_sparsely_and_match_a_full_update() {
    run(1000, Check::EachEdit);
}

/// The baseline run: `VAPORS_SPARSE_ROWS=3000 cargo nextest run ...`.
#[test]
fn the_requested_rows_update_sparsely_and_match_a_full_update() {
    let Ok(rows) = std::env::var("VAPORS_SPARSE_ROWS") else {
        eprintln!("VAPORS_SPARSE_ROWS is not set: the baseline run is off");
        return;
    };
    let rows = rows
        .parse()
        .unwrap_or_else(|_| panic!("VAPORS_SPARSE_ROWS={rows} is not a row count"));
    run(rows, Check::AfterSequence);
}
