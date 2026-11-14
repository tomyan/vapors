//! How `lane_flex_tests.rs` compares the column mirror with Taffy
//! (`lane_compare.rs`): each flex container is asked by the mirror
//! every way its parent asked it, and its answer and, for a layout,
//! each item's layout are compared, exactly.

use taffy::{Layout, LayoutOutput, NodeId, RunMode};

use crate::document::Document;
use crate::lane_decline::LaneDecline;
use crate::layout::asked::Asked;
use crate::layout::lane_block_answer::narrow;
use crate::layout::lane_compare::{
    Answered, Compared, answered, assert_answer, compare_each, taffy_answer,
};
use crate::layout::lane_flex::{Column, FlexSettled};
use crate::layout::lane_flex_answer::lane_flex_answer;
use crate::layout::lane_flex_summary::FlexSummary;
use crate::layout::lane_tree::Summary;
use crate::layout::tree::BoxTree;
use crate::layout::{Content, LayoutBox};

/// An item as the mirror laid it out: its slot, the run before it, its
/// leaf and its layout.
type LaidItem = (usize, FlexSummary, FlexSummary, Layout);

/// Compares every flex container with children in `doc` as laid out.
pub(super) fn compare(doc: &mut Document) -> Compared {
    compare_each(doc, is_flex_container, compare_ask)
}

/// Every flex container, with items or not: one without is asked of
/// the mirror too, which declines it (`NoItems`).
fn is_flex_container(layout_box: &LayoutBox) -> bool {
    matches!(layout_box.content, Content::Element(_))
        && layout_box.style.display == taffy::Display::Flex
}

/// Asks the container at `slot` one way and compares; the decline
/// when the mirror does not lay it out.
fn compare_ask(tree: &mut BoxTree<'_>, slot: usize, ask: &Asked) -> Result<Answered, LaneDecline> {
    let id = NodeId::from(slot);
    let taffy = taffy_answer(tree, id, ask);
    let (mine, short) = match tree.lane_column(id, ask.input) {
        FlexSettled::Short(short) => (short, true),
        FlexSettled::Declined(decline) => return Err(decline),
        FlexSettled::Laid(column) => (laid_answer(tree, slot, ask, &column)?, false),
    };
    let compared = assert_answer(slot, ask, &taffy, &mine);
    Ok(answered(compared, short, ask.input.run_mode))
}

/// The column's answer with every item laid out by the mirror, and
/// each item compared when the ask is a layout the column answered.
fn laid_answer(
    tree: &mut BoxTree<'_>,
    slot: usize,
    ask: &Asked,
    column: &Column,
) -> Result<LayoutOutput, LaneDecline> {
    let gap = f64::from(column.gap);
    let mut run = FlexSummary::default();
    let mut laid: Vec<LaidItem> = Vec::new();
    for child in tree.boxes[slot].children.clone() {
        let item = tree.lane_item(NodeId::from(child), column)?;
        let leaf = FlexSummary::leaf(item.leaf);
        laid.push((child, run, leaf, item.layout));
        run = run.combine(&leaf, gap);
    }
    let answer = lane_flex_answer(column, &run)?;
    if ask.input.run_mode == RunMode::PerformLayout {
        for item in &laid {
            assert_item_layout(tree, slot, column, item);
        }
    }
    Ok(answer)
}

/// The item's layout as the mirror gives it, its start along the
/// column added, against the layout Taffy wrote.
fn assert_item_layout(tree: &BoxTree<'_>, slot: usize, column: &Column, item: &LaidItem) {
    let &(child, left, leaf, mut layout) = item;
    let start = FlexSummary::start_of(&left, &leaf, f64::from(column.gap));
    layout.location.y += narrow(f64::from(column.content_box_inset.top) + start);
    let taffy = tree.boxes[child].layout;
    layout.order = taffy.order;
    assert_eq!(taffy, layout, "slot {slot}, item {child}");
}
