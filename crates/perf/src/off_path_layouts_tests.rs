//! The layout pages are what they say: cards wrap four to a line, the
//! grid has seven columns, paragraphs wrap into two to nine lines, and
//! the edited and inserted paragraph is the same at every size.

use vapors_elements::BoxRect;

use super::{GRID_CELLS, MIXED_TEXT, WRAPPED_CARDS};
use crate::off_path_page::{Shape, UnitPage, page_viewport};

/// The border boxes of the first `count` units of a 20-unit page.
fn boxes(shape: &Shape, count: usize) -> Vec<BoxRect> {
    let page = UnitPage::new(shape, 20).expect("mounts");
    let mut doc = page.doc.borrow_mut();
    let painted = doc.update(page_viewport()).expect("updates");
    page.units[..count]
        .iter()
        .map(|unit| painted.border_box(unit.element).expect("a box"))
        .collect()
}

/// Whether `a` and `b` are on one line: their tops within a hundredth
/// of a pixel.
fn level(a: &BoxRect, b: &BoxRect) -> bool {
    (a.y - b.y).abs() < 0.01
}

#[test]
fn cards_wrap_four_to_a_line() {
    let cards = boxes(&WRAPPED_CARDS, 5);
    assert!(
        cards[..4].iter().all(|card| level(card, &cards[0])),
        "{cards:?}"
    );
    assert!(cards[4].y > cards[0].y, "{cards:?}");
    assert!((cards[4].x - cards[0].x).abs() < 0.01, "{cards:?}");
}

#[test]
fn the_grid_has_seven_columns() {
    let cells = boxes(&GRID_CELLS, 8);
    assert!(
        cells[..7].iter().all(|cell| level(cell, &cells[0])),
        "{cells:?}"
    );
    assert!(cells[6].x > cells[0].x, "{cells:?}");
    assert!(cells[7].y > cells[0].y, "{cells:?}");
}

#[test]
fn paragraphs_wrap_into_two_to_nine_lines() {
    // Given: paragraphs 2 and 1 have one and five sentences a side
    let paragraphs = boxes(&MIXED_TEXT, 5);
    let (shortest, longest) = (paragraphs[2].height, paragraphs[1].height);

    // Then: two lines and nine lines of one line height
    let line = shortest / 2.0;
    assert!((longest / line - 9.0).abs() < 0.01, "{paragraphs:?}");
    assert!((15.0..30.0).contains(&line), "{paragraphs:?}");
}

#[test]
fn the_edited_and_the_inserted_paragraph_have_four_sentences_at_every_size() {
    for count in crate::off_path_kinds::SIZES {
        assert_eq!(super::sentences(count / 2), 4, "edited at {count}");
        assert_eq!(super::sentences(count), 4, "inserted at {count}");
    }
}
