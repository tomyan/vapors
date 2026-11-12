//! Owners under a grid parent (M10c): the flex tests' column with the
//! root displayed as a one-column grid, with and without row margins.

use taffy::Display;

use super::flex_tests::{
    LONGER, assert_laid_out_alone, column, column_with_margins, fresh, rebuilt,
};
use super::*;

#[test]
fn a_rebuilt_block_row_under_a_grid_that_answers_as_before_is_laid_out_alone() {
    // When: row 3 of a one-column grid is built again with the same text
    // Then
    assert_laid_out_alone(rebuilt(column(Display::Grid), "text"));
}

#[test]
fn a_rebuilt_row_with_a_vertical_margin_under_a_grid_that_answers_as_before_is_laid_out_alone() {
    // When: row 3 is built again with the same text
    // Then
    assert_laid_out_alone(rebuilt(column_with_margins(Display::Grid), "text"));
}

#[test]
fn a_rebuilt_block_row_under_a_grid_that_grows_has_the_grid_laid_out_again() {
    // When: row 3 is built again with a text that wraps
    let (settled, laid_out, (_, after)) = rebuilt(column(Display::Grid), LONGER);

    // Then
    let expected = Settled {
        resized: 1,
        ..Settled::default()
    };
    assert_eq!(settled, expected);
    assert!(laid_out.contains(&0), "the root is laid out again");
    assert_eq!(
        after,
        fresh(column(Display::Grid), LONGER),
        "the layout of a fresh tree"
    );
}
