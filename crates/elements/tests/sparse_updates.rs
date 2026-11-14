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
use vapors_elements::{Document, UpdateCounts};

/// What one update after an edit did, as structural counts.
#[derive(Debug)]
struct Work {
    restyled: usize,
    laid_out: usize,
    text_layouts: usize,
    walk: UpdateCounts,
}

impl Work {
    fn of(doc: &mut Document) -> Self {
        doc.update(rows_viewport())
            .unwrap_or_else(|errors| panic!("refused: {errors:#?}"));
        Self {
            restyled: doc.elements_restyled(),
            laid_out: doc.boxes_laid_out(),
            text_layouts: doc.text_layouts_built(),
            walk: doc.update_counts(),
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
const EXPECTED: [(usize, usize, usize); 6] = [
    (0, 2, 1),
    (0, 9, 2),
    (0, 0, 0),
    (4, 0, 0),
    (1, 0, 0),
    (4, 0, 0),
];

/// The walk after each edit of `EDITS`, by row count: elements visited,
/// boxes built, layout calls (the same-size edit's are constant in
/// rows: the rebuilt value is laid out in isolation, M10,
/// `layout/isolated.rs`; the resizing edit's are 13 whatever the rows:
/// the body's lane asks the resized row only, `layout/lane_layout.rs`;
/// before the lane they were rows + 12, the body asking each row),
/// boxes painted, boxes copied (kept parts
/// read and kept: none, as no edit moves a box it did not build), scene
/// items built, map
/// entries patched (the elements built again or moved), paint order
/// and scroll chain walks (none: no edit changes the tree's shape),
/// clips redone (the boxes built again or moved), stackings made (the
/// boxes built again), slots chained before the placement walk (the
/// in-flow children of the boxes built again or laid out: the body's
/// rows when the body is laid out), boxes marked kept (the boxes built
/// again). A row is
/// 4 elements and 7 boxes (3 text runs) and draws 4 items (the button's
/// quad and 3 texts), 5 with the focus outline, 6 with the outline and
/// the marked row's background; `html` and `body` add 2
/// elements and 2 boxes. An update that builds no box and moves none
/// paints nothing (`kept_paint.rs`); any other paints only the boxes it
/// built or moved, keeping the rest in place (`kept_box.rs`): the longer label
/// moves the 4 boxes after it in its row. The second table in
/// docs/plans/iu1.md is these numbers.
const EXPECTED_WALKS: [(usize, [[usize; 13]; 6]); 2] = [
    (
        100,
        [
            [1, 2, 3, 2, 0, 1, 1, 0, 0, 2, 2, 1, 2],
            [1, 2, 13, 6, 0, 4, 3, 0, 0, 6, 2, 105, 2],
            [0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            [4, 2, 1, 2, 0, 2, 1, 0, 0, 2, 2, 1, 2],
            [1, 2, 1, 2, 0, 3, 1, 0, 0, 2, 2, 1, 2],
            [4, 7, 1, 7, 0, 5, 4, 0, 0, 7, 7, 6, 7],
        ],
    ),
    (
        1000,
        [
            [1, 2, 3, 2, 0, 1, 1, 0, 0, 2, 2, 1, 2],
            [1, 2, 13, 6, 0, 4, 3, 0, 0, 6, 2, 1005, 2],
            [0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            [4, 2, 1, 2, 0, 2, 1, 0, 0, 2, 2, 1, 2],
            [1, 2, 1, 2, 0, 3, 1, 0, 0, 2, 2, 1, 2],
            [4, 7, 1, 7, 0, 5, 4, 0, 0, 7, 7, 6, 7],
        ],
    ),
];

fn run(rows: usize, check: Check) {
    run_on(&rows_page(rows), rows, check);
}

fn run_on(page: &RowsPage, rows: usize, check: Check) {
    for (index, (edit, expected)) in EDITS.into_iter().zip(EXPECTED).enumerate() {
        let work = edited(page, edit);
        eprintln!("{rows:>5} rows  {edit:<16?} {work:?}");
        assert_counts(rows, edit, &work, expected);
        assert_walk(rows, index, edit, &work);
        if check == Check::EachEdit {
            assert_oracle(page, rows, &format!("{edit:?}"));
        }
    }
    if check == Check::AfterSequence {
        assert_oracle(page, rows, &format!("{EDITS:?}"));
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
/// count. Whether an edit reports a change is not asserted: an equal
/// write is reported as one today, and IU's direction will change that.
fn assert_counts(rows: usize, edit: Edit, work: &Work, expected: (usize, usize, usize)) {
    let actual = (work.restyled, work.laid_out, work.text_layouts);
    assert_eq!(actual, expected, "{rows} rows, {edit:?}: {work:?}; {FIX}");
}

/// The walk counts are exactly `EXPECTED_WALKS`' at the row counts it
/// lists, and reported at any other.
fn assert_walk(rows: usize, index: usize, edit: Edit, work: &Work) {
    let Some((_, walks)) = EXPECTED_WALKS.iter().find(|(at, _)| *at == rows) else {
        return;
    };
    let c = work.walk;
    let actual = [
        c.elements_visited,
        c.boxes_built,
        c.layout_calls,
        c.boxes_painted,
        c.parts_visited_kept,
        c.scene_items_built,
        c.map_entries_patched,
        c.order_walks,
        c.chain_walks,
        c.clips_redone,
        c.stackings_computed,
        c.slots_chained,
        c.boxes_kept,
    ];
    assert_eq!(
        actual, walks[index],
        "{rows} rows, {edit:?}: {work:?}; {FIX}"
    );
}

const FIX: &str = "update EXPECTED or EXPECTED_WALKS and docs/plans/iu1.md; reproduce: \
    cargo nextest run -p vapors-elements --test sparse_updates --no-capture";

#[test]
fn a_hundred_rows_update_sparsely_and_match_a_full_update() {
    run(100, Check::EachEdit);
}

#[test]
fn a_thousand_rows_update_sparsely_and_match_a_full_update() {
    let page = rows_page(1000);
    // The debug layout check is skipped here: this test's oracle
    // compares the same layout with a full update, and with both the
    // test took 44 s of its 60 s limit (17.6 s without).
    // The oracle runs once, after the sequence: each run at 1000 rows
    // is a snapshot of the kept update and one of a full update, 3 to 5 s
    // each in a debug build at load 18 (measured 2026-11-12), so 6 runs
    // took most of the test's 48 s; with one it takes 8 s. The counters are still asserted after
    // every edit, and `a_hundred_rows_...` runs the oracle after each.
    // Retire this when the oracle is cheaper, or when it runs in a tier
    // of its own.
    page.doc.borrow_mut().skip_layout_check();
    run_on(&page, 1000, Check::AfterSequence);
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
