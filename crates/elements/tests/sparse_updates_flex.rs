//! A text edit in a flex-column list (M10c): the edited row, a flex
//! item its parent asks for its size, is laid out in isolation and
//! the layout calls do not grow with rows, with or without a margin
//! on the rows; an edit that makes the row taller lays the column out
//! again. Each update is checked by the correctness oracle.

mod support;

use support::flex_list::{FlexList, ROW_MARGIN, flex_list, list_viewport};
use support::oracle::assert_matches_full;
use vapors_elements::UpdateCounts;

/// Wraps in the 640px viewport, so a row of `GROWS` grows taller.
const LONGER: &str = "a text long enough to wrap onto a second line in the viewport the list is \
    laid out in, and then some more, so that it wraps however the font measures it";

/// Rows as tall as their text, with the margin.
const GROWS: &str = "height: auto; margin: 4px 0;";

const FIX: &str = "update sparse_updates_flex.rs and docs/plans/iu1.md; reproduce: \
    cargo nextest run -p vapors-elements --test sparse_updates_flex --no-capture";

/// The counts of one update after `edit`, checked by the oracle.
fn edited(list: &FlexList, edit: impl FnOnce(&FlexList), what: &str) -> UpdateCounts {
    list.doc.borrow_mut().take_changed();
    edit(list);
    let counts = {
        let mut doc = list.doc.borrow_mut();
        doc.update(list_viewport())
            .unwrap_or_else(|errors| panic!("refused: {errors:#?}"));
        doc.update_counts()
    };
    assert_matches_full(&list.doc, list_viewport(), what);
    counts
}

fn owners(counts: &UpdateCounts) -> (usize, usize, usize) {
    (
        counts.owners_isolated,
        counts.owners_resized,
        counts.owners_uncompared,
    )
}

#[test]
fn a_same_size_edit_lays_out_the_row_alone_at_any_row_count() {
    for extra in ["", ROW_MARGIN] {
        for rows in [100, 1000] {
            // Given
            let list = flex_list(rows, extra);

            // When
            let what = format!("{rows} rows, {extra:?}, value edit");
            let counts = edited(&list, |list| list.set_middle_value(99_999), &what);

            // Then: the row, its text and the calls above them; 5 at
            // any row count
            assert_eq!(owners(&counts), (1, 0, 0), "{what}: {counts:?}; {FIX}");
            assert_eq!(counts.layout_calls, 5, "{what}: {counts:?}; {FIX}");
            eprintln!("{rows:>5} rows {extra:<16} {counts:?}");
        }
    }
}

#[test]
fn an_edit_that_makes_the_row_taller_lays_out_that_row_whatever_the_rows() {
    let calls = |rows: usize| {
        // Given
        let list = flex_list(rows, GROWS);

        // When
        let what = format!("{rows} rows, taller edit");
        let counts = edited(&list, |list| list.set_middle_text(LONGER), &what);

        // Then: the row's answer differs, so the column is laid out
        // again, by its lane: the taller row and its ancestors only
        assert_eq!(owners(&counts), (0, 1, 0), "{what}: {counts:?}; {FIX}");
        assert!(counts.lanes >= 1, "{what}: {counts:?}; {FIX}");
        counts.layout_calls
    };
    assert_eq!(
        calls(100),
        calls(1000),
        "layout calls grow with the rows; {FIX}"
    );
}
