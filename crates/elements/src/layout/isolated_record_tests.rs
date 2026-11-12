//! The record of a size query's answer (`asked.rs`) and its bound
//! (`tree_cache.rs`): what a box keeps, a box answering otherwise
//! than it recorded, and a box asked more ways than it records.

use taffy::{
    AxisStaticAlign, AxisStaticEdge, AxisStaticPosition, Display, LayoutOutput, Line, OofCandidate,
    Point, Position, RunMode, Size,
};

use super::flex_tests::column_with_margins;
use super::tests::{ROW, Texts, changed_row, rebuild_row};
use crate::layout::asked::SizeAnswer;
use crate::layout::tree::cache::ASKED_LIMIT;
use crate::layout::{Calcs, Placed, TextCache, lay_out};

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

#[test]
fn a_size_answer_bubbling_an_out_of_flow_box_up_is_not_recorded() {
    // Given
    let mut output = LayoutOutput::from_outer_size(Size {
        width: 1.0,
        height: 1.0,
    });
    let align = AxisStaticAlign {
        keyword: AxisStaticEdge::Start,
        safety: taffy::AlignmentSafety::Unsafe,
        fallback: AxisStaticEdge::Start,
    };
    output.oof_candidates.push(OofCandidate {
        node: taffy::NodeId::from(1usize),
        order: 0,
        position: Position::Absolute,
        static_position: Point {
            x: AxisStaticPosition {
                area: Line {
                    start: 0.0,
                    end: 1.0,
                },
                align,
            },
            y: AxisStaticPosition {
                area: Line {
                    start: 0.0,
                    end: 1.0,
                },
                align,
            },
        },
    });

    // When / Then
    assert_eq!(SizeAnswer::of(&output), None);
}

/// Row 3's record of its size answer set to another.
fn tamper(placed: &mut Placed) {
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
}

#[test]
fn a_box_answering_a_recorded_size_query_otherwise_has_the_record_disowned_and_counted() {
    // Given: row 3's record tampered with, and the root's cache cleared
    // so it asks the row again
    let mut texts = Texts::new();
    let (mut placed, _) = texts.lay_out(column_with_margins(Display::Flex), Vec::new());
    tamper(&mut placed);
    placed.boxes[0].cache.clear();

    // When: the row answers as it did, which is not what it recorded
    let (mut placed, work) = texts.lay_out(placed, Vec::new());

    // Then: counted, and the row built again is not compared
    assert_eq!(work.disowned, 1);
    let old = rebuild_row(&mut placed, "text");
    placed.boxes[ROW].style = old.style.clone();
    let (_, work) = texts.lay_out(placed, changed_row(Some(old)));
    assert_eq!(
        (work.disowned, work.settled.uncompared),
        (0, 1),
        "{:?}",
        work.settled
    );
}

/// `placed` laid out at each of `widths`.
fn laid_out_at(texts: &mut Texts, mut placed: Placed, widths: impl Iterator<Item = u32>) -> Placed {
    for width in widths {
        #[allow(clippy::cast_precision_loss)] // widths of a few hundred pixels
        let size = (width as f32, 100.0);
        let (fonts, cache): (_, &mut TextCache) = (&mut texts.0, &mut texts.1);
        placed = lay_out(
            placed,
            (fonts, cache),
            1.0,
            size,
            &Calcs::default(),
            Vec::new(),
        )
        .unwrap_or_else(|failed| panic!("refused: {}", failed.0))
        .0;
    }
    placed
}

#[test]
fn a_box_asked_more_ways_than_it_records_is_not_compared_until_its_cache_is_cleared() {
    // Given: a flex-column row is asked three ways per width, so six
    // widths pass the limit of 16
    let mut texts = Texts::new();
    let mut placed = laid_out_at(&mut texts, column_with_margins(Display::Flex), 200..206);
    assert!(
        placed.boxes[ROW].asked.is_empty() && placed.boxes[ROW].asked_overflowed,
        "the record is abandoned: {} ways",
        placed.boxes[ROW].asked.len()
    );

    // When: row 3 is built again as it was
    let old = rebuild_row(&mut placed, "text");
    placed.boxes[ROW].style = old.style.clone();
    let (mut placed, work) = texts.lay_out(placed, changed_row(Some(old)));

    // Then: not compared; a cleared cache records again
    assert_eq!(work.settled.uncompared, 1, "{:?}", work.settled);
    placed.boxes[ROW].cache.clear();
    placed.boxes[ROW].asked_overflowed = false;
    placed.boxes[0].cache.clear();
    let (placed, _) = texts.lay_out(placed, Vec::new());
    let ways = placed.boxes[ROW].asked.len();
    assert!((1..=ASKED_LIMIT).contains(&ways), "{ways} ways recorded");
}
