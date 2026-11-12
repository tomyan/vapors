//! Owners under a flex or grid parent (M10c). Such a parent asks a
//! child for its size (`ComputeSize`) before it lays it out. A block
//! row answers with its margins, and Taffy's cache keeps no such
//! answer, so the box records what its parent read (`asked.rs`) and
//! the replay compares with that: an unchanged block row under a flex
//! column or a grid is laid out alone, with or without a margin.

use taffy::{Display, FlexDirection, LengthPercentageAuto, Rect};

use super::tests::{ROW, ROW_TEXT, Texts, changed_row, layouts, rebuild_row, rows, sorted};
use super::*;
use crate::layout::Placed;
use crate::layout::asked::SizeAnswer;

/// Wraps at the 200px the tests lay out in, so the row grows taller.
const LONGER: &str = "a text long enough to wrap onto a second line in the width laid out";

/// The vertical margin of a row of `column_with_margins`: a block
/// answers a size query with its margins, which Taffy's cache keeps no
/// entry for.
const MARGIN: f32 = 4.0;

/// The `rows` tree with block rows under a root of `display`, laid
/// out as a column.
fn column(display: Display) -> Placed {
    let mut placed = rows();
    placed.boxes[0].style.display = display;
    placed.boxes[0].style.flex_direction = FlexDirection::Column;
    for row in [1, ROW, 5] {
        placed.boxes[row].style.display = Display::Block;
    }
    placed
}

/// `column`, its rows with a vertical margin of `MARGIN`.
pub(super) fn column_with_margins(display: Display) -> Placed {
    let mut placed = column(display);
    for row in [1, ROW, 5] {
        placed.boxes[row].style.margin = vertical_margin();
    }
    placed
}

fn vertical_margin() -> Rect<LengthPercentageAuto> {
    Rect {
        top: LengthPercentageAuto::length(MARGIN),
        bottom: LengthPercentageAuto::length(MARGIN),
        ..Rect::auto()
    }
}

/// A fresh block box for row 3 holding `text`, in its slot, styled
/// as the row it replaces; what the old box held.
fn rebuild_block_row(placed: &mut Placed, text: &str) -> Before {
    let old = rebuild_row(placed, text);
    placed.boxes[ROW].style.display = Display::Block;
    placed.boxes[ROW].style.margin = old.style.margin;
    old
}

/// Row 3 of `column` built again with `text`; what it settled as, the
/// slots laid out, and the layouts before and after.
fn rebuilt(
    column: Placed,
    text: &str,
) -> (
    Settled,
    Vec<usize>,
    (Vec<taffy::Layout>, Vec<taffy::Layout>),
) {
    let mut texts = Texts::new();
    let (mut placed, _) = texts.lay_out(column, Vec::new());
    let before = layouts(&placed);
    let old = rebuild_block_row(&mut placed, text);
    let (placed, work) = texts.lay_out(placed, changed_row(Some(old)));
    (
        work.settled,
        sorted(work.written.laid_out),
        (before, layouts(&placed)),
    )
}

/// The layouts of a fresh `column` whose row 3 holds `text`.
fn fresh(mut column: Placed, text: &str) -> Vec<taffy::Layout> {
    let mut texts = Texts::new();
    rebuild_block_row(&mut column, text);
    layouts(&texts.lay_out(column, Vec::new()).0)
}

/// What `settled` must be for a row laid out alone, which lays out
/// only the row and its text and leaves every layout as it was.
fn assert_laid_out_alone(
    (settled, laid_out, (before, after)): (
        Settled,
        Vec<usize>,
        (Vec<taffy::Layout>, Vec<taffy::Layout>),
    ),
) {
    let expected = Settled {
        isolated: 1,
        ..Settled::default()
    };
    assert_eq!(settled, expected);
    assert_eq!(laid_out, [ROW, ROW_TEXT], "only the row");
    assert_eq!(after, before, "every layout as before");
}

#[test]
fn a_rebuilt_block_row_under_a_flex_column_that_answers_as_before_is_laid_out_alone() {
    // Given: the flex column asked row 3 for its size, then laid it out
    let mut texts = Texts::new();
    let (placed, _) = texts.lay_out(column(Display::Flex), Vec::new());
    let modes: Vec<RunMode> = placed.boxes[ROW]
        .asked
        .iter()
        .map(|asked| asked.input.run_mode)
        .collect();
    assert!(
        modes.contains(&RunMode::ComputeSize) && modes.contains(&RunMode::PerformLayout),
        "the flex column asks a row for its size and lays it out: {modes:?}"
    );

    // When: row 3 is built again with the same text
    // Then
    assert_laid_out_alone(rebuilt(column(Display::Flex), "text"));
}

#[test]
fn a_rebuilt_block_row_under_a_flex_column_that_grows_has_the_column_laid_out_again() {
    // When: row 3 is built again with a text that wraps
    let (settled, laid_out, (_, after)) = rebuilt(column(Display::Flex), LONGER);

    // Then
    let expected = Settled {
        resized: 1,
        ..Settled::default()
    };
    assert_eq!(settled, expected);
    assert!(laid_out.contains(&0), "the root is laid out again");
    assert_eq!(
        after,
        fresh(column(Display::Flex), LONGER),
        "the layout of a fresh tree"
    );
}

#[test]
fn a_rebuilt_row_with_a_vertical_margin_under_a_flex_column_that_answers_as_before_is_laid_out_alone()
 {
    // Given: the column asked row 3 for its size, which it answered
    // with its margins; the cache keeps no entry for such an answer,
    // so the box records it
    let mut texts = Texts::new();
    let (placed, _) = texts.lay_out(column_with_margins(Display::Flex), Vec::new());
    let recorded: Vec<Option<SizeAnswer>> = placed.boxes[ROW]
        .asked
        .iter()
        .map(|asked| asked.answered)
        .collect();
    assert!(
        recorded.iter().any(Option::is_some),
        "a size query's answer is recorded: {recorded:?}"
    );

    // When: row 3 is built again with the same text
    // Then
    assert_laid_out_alone(rebuilt(column_with_margins(Display::Flex), "text"));
}

#[test]
fn a_rebuilt_row_with_a_vertical_margin_under_a_flex_column_that_grows_has_the_column_laid_out_again()
 {
    // When: row 3 is built again with a text that wraps
    let (settled, laid_out, (_, after)) = rebuilt(column_with_margins(Display::Flex), LONGER);

    // Then
    let expected = Settled {
        resized: 1,
        ..Settled::default()
    };
    assert_eq!(settled, expected);
    assert!(laid_out.contains(&0), "the root is laid out again");
    assert_eq!(
        after,
        fresh(column_with_margins(Display::Flex), LONGER),
        "the layout of a fresh tree"
    );
}

#[cfg(feature = "grid")]
#[test]
fn a_rebuilt_block_row_under_a_grid_that_answers_as_before_is_laid_out_alone() {
    // When: row 3 of a one-column grid is built again with the same text
    // Then
    assert_laid_out_alone(rebuilt(column(Display::Grid), "text"));
}

#[cfg(feature = "grid")]
#[test]
fn a_rebuilt_row_with_a_vertical_margin_under_a_grid_that_answers_as_before_is_laid_out_alone() {
    // When: row 3 is built again with the same text
    // Then
    assert_laid_out_alone(rebuilt(column_with_margins(Display::Grid), "text"));
}

#[cfg(feature = "grid")]
#[test]
fn a_rebuilt_block_row_under_a_grid_that_grows_has_the_grid_laid_out_again() {
    // When: row 3 is built again with a text that wraps
    let (settled, laid_out, (_, after)) = rebuilt(column(Display::Grid), LONGER);

    // Then
    assert_eq!(settled.isolated, 0, "{settled:?}");
    assert!(laid_out.contains(&0), "the root is laid out again");
    assert_eq!(
        after,
        fresh(column(Display::Grid), LONGER),
        "the layout of a fresh tree"
    );
}
