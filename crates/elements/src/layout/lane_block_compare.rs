//! How `lane_block_tests.rs` compares the block mirror with Taffy
//! (`lane_compare.rs`): each block container is asked by the mirror
//! every way its parent asked it, and its answer and each child's
//! layout are compared, exactly.

use taffy::{LayoutOutput, NodeId, RunMode};

use crate::document::Document;
use crate::lane_decline::LaneDecline;
use crate::layout::asked::Asked;
use crate::layout::lane_block::{Container, Settled};
use crate::layout::lane_block_answer::lane_answer;
use crate::layout::lane_block_summary::BlockSummary;
use crate::layout::lane_compare::{
    Answered, Compared, answered, assert_answer, compare_each, taffy_answer,
};
use crate::layout::lane_tree::Summary;
use crate::layout::tree::BoxTree;
use crate::layout::{Content, LayoutBox};

/// Compares every block container with children in `doc` as laid out.
pub(super) fn compare(doc: &mut Document) -> Compared {
    compare_each(doc, is_block_container, compare_ask)
}

fn is_block_container(layout_box: &LayoutBox) -> bool {
    matches!(layout_box.content, Content::Element(_))
        && layout_box.style.display == taffy::Display::Block
        && !layout_box.children.is_empty()
}

/// Asks the container at `slot` one way and compares; the decline
/// when the mirror does not lay it out.
fn compare_ask(tree: &mut BoxTree<'_>, slot: usize, ask: &Asked) -> Result<Answered, LaneDecline> {
    let id = NodeId::from(slot);
    let taffy = taffy_answer(tree, id, ask);
    let (mine, short) = match tree.lane_container(id, ask.input, ask.in_block) {
        Settled::Short(short) => (short, true),
        Settled::Declined(decline) => return Err(decline),
        Settled::Laid(container) => (laid_answer(tree, slot, ask, &container)?, false),
    };
    let compared = assert_answer(slot, ask, &taffy, &mine);
    Ok(answered(compared, short, ask.input.run_mode))
}

/// The container's answer with every child laid out by the mirror and
/// compared when the ask is a layout.
fn laid_answer(
    tree: &mut BoxTree<'_>,
    slot: usize,
    ask: &Asked,
    container: &Container,
) -> Result<LayoutOutput, LaneDecline> {
    let children = tree.boxes[slot].children.clone();
    let mut run = BlockSummary::default();
    for child in children {
        let laid = tree.lane_child(NodeId::from(child), container)?;
        let leaf = BlockSummary::leaf(laid.leaf)?;
        if ask.input.run_mode == RunMode::PerformLayout {
            assert_child_layout(tree, (slot, child), (container, &run, &leaf), laid.layout);
        }
        run = run.combine(&leaf, 0.0);
    }
    lane_answer(container, &run)
}

/// The child's layout as the mirror gives it, its start along the axis
/// added, against the layout Taffy wrote.
fn assert_child_layout(
    tree: &BoxTree<'_>,
    (slot, child): (usize, usize),
    (container, left, leaf): (&Container, &BlockSummary, &BlockSummary),
    mut layout: taffy::Layout,
) {
    let escape = if container.own_margins_collapse.start {
        left.combine(leaf, 0.0).top().resolve()
    } else {
        0.0
    };
    let start = BlockSummary::start_of(left, leaf, 0.0) - escape;
    let top = f64::from(container.resolved_inset.top) + start;
    layout.location.y += crate::layout::lane_block_answer::narrow(top);
    let taffy = tree.boxes[child].layout;
    layout.order = taffy.order;
    assert_eq!(taffy, layout, "slot {slot}, child {child}");
}
