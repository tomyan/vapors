//! How `lane_block_tests.rs` compares the mirror with Taffy: on a page
//! laid out by Taffy, each block container is asked by the mirror
//! every way its parent asked it, and the container's answer and each
//! child's layout are compared, exactly. Every ask is accounted for:
//! answered short, as a size query, as a layout, declined by name, or
//! with no recorded answer to compare against.

use taffy::{BlockFormattingContext, LayoutInput, LayoutOutput, NodeId, RunMode};

use super::page::VIEWPORT;
use crate::document::Document;
use crate::lane_decline::LaneDecline;
use crate::layout::asked::{Asked, SizeAnswer};
use crate::layout::lane_block::{Container, Settled};
use crate::layout::lane_block_answer::lane_answer;
use crate::layout::lane_block_summary::BlockSummary;
use crate::layout::lane_tree::Summary;
use crate::layout::tree::BoxTree;
use crate::layout::{Content, LayoutBox, Written};

/// How every ask of every block container came out.
#[derive(Debug, Default, PartialEq)]
pub(super) struct Compared {
    /// Containers every ask of which the mirror answered as Taffy did.
    pub(super) containers: usize,
    /// Asks answered from the known dimensions alone.
    pub(super) short: usize,
    /// Size queries whose recorded answer matched.
    pub(super) size_asks: usize,
    /// Layouts whose whole answer and children's layouts matched.
    pub(super) layout_asks: usize,
    /// Size queries with no recorded answer to compare against (the
    /// answer bubbled an out-of-flow box, or was disowned).
    pub(super) unanswered: usize,
    /// Every ask declined, by reason, in ask order.
    pub(super) declined: Vec<LaneDecline>,
}

/// How one ask came out when the mirror answered it.
enum Answered {
    Short,
    Size,
    Layout,
    Unanswered,
}

/// Compares every block container with children in `doc` as laid out.
pub(super) fn compare(doc: &mut Document) -> Compared {
    let kept = doc.kept.as_ref().expect("updated");
    let mut placed = std::mem::take(&mut doc.boxes.placed);
    let mut tree = BoxTree {
        boxes: placed.boxes,
        fonts: &mut doc.fonts,
        cache: &mut doc.texts,
        scale: VIEWPORT.scale,
        calcs: &kept.calcs,
        written: Written::default(),
        calls: 0,
        disowned: 0,
        lanes: &mut placed.lanes,
        lanes_enabled: false,
        stale: rustc_hash::FxHashSet::default(),
        lane_counts: crate::layout::lane_layout::LaneCounts::default(),
    };
    let mut compared = Compared::default();
    for slot in 0..tree.boxes.len() {
        if is_block_container(&tree.boxes[slot]) {
            compare_container(&mut tree, slot, &mut compared);
        }
    }
    compared
}

fn is_block_container(layout_box: &LayoutBox) -> bool {
    matches!(layout_box.content, Content::Element(_))
        && layout_box.style.display == taffy::Display::Block
        && !layout_box.children.is_empty()
}

/// Every ask of the container at `slot` compared and accounted for.
fn compare_container(tree: &mut BoxTree<'_>, slot: usize, compared: &mut Compared) {
    let mut every_ask_answered = true;
    for ask in tree.boxes[slot].asked.clone() {
        match compare_ask(tree, slot, &ask) {
            Ok(Answered::Short) => compared.short += 1,
            Ok(Answered::Size) => compared.size_asks += 1,
            Ok(Answered::Layout) => compared.layout_asks += 1,
            Ok(Answered::Unanswered) => compared.unanswered += 1,
            Err(decline) => {
                every_ask_answered = false;
                compared.declined.push(decline);
            }
        }
    }
    if every_ask_answered {
        compared.containers += 1;
    }
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
    Ok(match (compared, short, ask.input.run_mode) {
        (false, _, _) => Answered::Unanswered,
        (true, true, _) => Answered::Short,
        (true, false, RunMode::PerformLayout) => Answered::Layout,
        (true, false, _) => Answered::Size,
    })
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

/// Taffy's answer to `ask`, from the container's cache.
fn taffy_answer(tree: &mut BoxTree<'_>, id: NodeId, ask: &Asked) -> LayoutOutput {
    if !ask.in_block {
        return tree.lay_out(id, ask.input, None);
    }
    let mut formatting = BlockFormattingContext::new();
    let mut root = formatting.root_block_context();
    let mut context = root.sub_context(0.0, [0.0, 0.0]);
    tree.lay_out(id, ask.input, Some(&mut context))
}

/// A layout's answer whole; a size query's as the parent reads it.
/// False when nothing was recorded to compare against.
fn assert_answer(slot: usize, ask: &Asked, taffy: &LayoutOutput, mine: &LayoutOutput) -> bool {
    let input: LayoutInput = ask.input;
    match input.run_mode {
        RunMode::PerformLayout => assert_eq!(taffy, mine, "slot {slot}, layout {input:?}"),
        RunMode::ComputeSize => {
            let Some(recorded) = ask.answered else {
                return false;
            };
            assert_eq!(
                SizeAnswer::of(mine),
                Some(recorded),
                "slot {slot}, size {input:?}"
            );
        }
        RunMode::PerformHiddenLayout => unreachable!("a hidden layout is not asked of a lane"),
    }
    true
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
