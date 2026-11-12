//! The states that lay the ancestors out again (`isolated.rs`).

use taffy::{LayoutInput, RunMode};

use super::tests::{ROW, Texts, changed_row, layouts, rebuild_row, rows};
use super::*;

#[test]
fn a_rebuilt_row_that_answers_otherwise_has_its_ancestors_laid_out_again() {
    // Given: row 3 built again with a longer text, and a fresh tree
    // with that text for the layout to match
    let mut texts = Texts::new();
    let (mut placed, _) = texts.lay_out(rows(), Vec::new());
    let old = rebuild_row(&mut placed, "a much longer text than before");
    let mut fresh = rows();
    rebuild_row(&mut fresh, "a much longer text than before");
    let (fresh, _) = texts.lay_out(fresh, Vec::new());

    // When
    let (placed, work) = texts.lay_out(placed, changed_row(Some(old)));

    // Then
    let expected = Settled {
        resized: 1,
        ..Settled::default()
    };
    assert_eq!(work.settled, expected);
    assert!(
        work.written.laid_out.contains(&0),
        "the root is laid out again"
    );
    assert_eq!(
        layouts(&placed),
        layouts(&fresh),
        "the layout of a fresh tree"
    );
}

#[test]
fn a_rebuilt_row_with_nothing_to_compare_against_has_its_ancestors_laid_out_again() {
    // Given: row 3 built again, and what its slot held is not known
    let mut texts = Texts::new();
    let (mut placed, _) = texts.lay_out(rows(), Vec::new());
    let before = layouts(&placed);
    rebuild_row(&mut placed, "text");

    // When
    let (placed, work) = texts.lay_out(placed, changed_row(None));

    // Then
    let expected = Settled {
        uncompared: 1,
        ..Settled::default()
    };
    assert_eq!(work.settled, expected);
    assert!(
        work.written.laid_out.contains(&0),
        "the root is laid out again"
    );
    assert_eq!(layouts(&placed), before);
}

#[test]
fn a_rebuilt_row_whose_old_box_was_never_asked_has_its_ancestors_laid_out_again() {
    // Given: the old box's cache holds nothing it was asked
    let mut texts = Texts::new();
    let (mut placed, _) = texts.lay_out(rows(), Vec::new());
    let mut old = rebuild_row(&mut placed, "text");
    old.asked.clear();

    // When
    let (_, work) = texts.lay_out(placed, changed_row(Some(old)));

    // Then
    assert_eq!(work.settled.uncompared, 1);
    assert!(
        work.written.laid_out.contains(&0),
        "the root is laid out again"
    );
}

#[test]
fn an_input_the_old_cache_no_longer_holds_has_the_ancestors_laid_out_again() {
    // Given: the old box was asked an input its cache does not hold
    let mut texts = Texts::new();
    let (mut placed, _) = texts.lay_out(rows(), Vec::new());
    let mut old = rebuild_row(&mut placed, "text");
    old.asked.push(Asked {
        input: LayoutInput {
            run_mode: RunMode::PerformLayout,
            ..LayoutInput::HIDDEN
        },
        in_block: false,
    });

    // When
    let (_, work) = texts.lay_out(placed, changed_row(Some(old)));

    // Then
    assert_eq!(work.settled.uncompared, 1);
    assert!(
        work.written.laid_out.contains(&0),
        "the root is laid out again"
    );
}

#[test]
fn an_absolute_row_is_not_compared_and_has_its_ancestors_laid_out_again() {
    // Given: row 3 is absolutely positioned, before and after
    let mut texts = Texts::new();
    let mut placed = rows();
    placed.boxes[ROW].style.position = taffy::Position::Absolute;
    let (mut placed, _) = texts.lay_out(placed, Vec::new());
    let old = rebuild_row(&mut placed, "text");
    placed.boxes[ROW].style.position = taffy::Position::Absolute;

    // When
    let (_, work) = texts.lay_out(placed, changed_row(Some(old)));

    // Then
    assert_eq!(work.settled.uncompared, 1);
    assert!(
        work.written.laid_out.contains(&0),
        "the root is laid out again"
    );
}
