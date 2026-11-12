//! The record of a size query's answer (`asked.rs`): what a box keeps,
//! and the debug check that a box asked the same way answers the same
//! until its cache is cleared.

use taffy::{Display, LayoutOutput, RunMode, Size};

use super::flex_tests::column_with_margins;
use super::tests::{ROW, Texts};
use crate::layout::asked::SizeAnswer;

#[test]
fn a_block_row_records_its_answer_to_a_size_query_and_none_to_a_layout() {
    // Given: block rows with a margin under a flex column
    let mut texts = Texts::new();

    // When
    let (placed, _) = texts.lay_out(column_with_margins(Display::Flex), Vec::new());

    // Then: every size query answered is recorded, no layout is
    let recorded: Vec<(RunMode, bool)> = placed.boxes[ROW]
        .asked
        .iter()
        .map(|asked| (asked.input.run_mode, asked.answered.is_some()))
        .collect();
    assert!(
        recorded.contains(&(RunMode::ComputeSize, true)),
        "a size query's answer is recorded: {recorded:?}"
    );
    assert!(
        recorded
            .iter()
            .all(|&(mode, answered)| answered == (mode == RunMode::ComputeSize)),
        "{recorded:?}"
    );
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "answered the same size query otherwise")]
fn a_box_answering_a_recorded_size_query_otherwise_is_caught() {
    // Given: row 3's record of its size answer tampered with, and the
    // root's cache cleared so it asks the row again
    let mut texts = Texts::new();
    let (mut placed, _) = texts.lay_out(column_with_margins(Display::Flex), Vec::new());
    let recorded = placed.boxes[ROW]
        .asked
        .iter_mut()
        .find(|asked| asked.answered.is_some())
        .expect("a size query recorded");
    let other = LayoutOutput::from_outer_size(Size {
        width: 1.0,
        height: 1.0,
    });
    recorded.answered = SizeAnswer::of(&other);
    placed.boxes[0].cache.clear();

    // When: the row answers as it did, which is not what it recorded
    texts.lay_out(placed, Vec::new());
}
