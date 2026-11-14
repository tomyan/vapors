//! How the lane tests compare a mirror with Taffy (`lane_block_compare.rs`,
//! `lane_flex_compare.rs`): on a page laid out by Taffy, each container
//! of a kind is asked by the mirror every way its parent asked it, and
//! every ask is accounted for: answered short, as a size query, as a
//! layout, declined by name, or with no recorded answer to compare
//! against.

use taffy::{BlockFormattingContext, LayoutInput, LayoutOutput, NodeId, RunMode};

use crate::document::Document;
use crate::lane_decline::LaneDecline;
use crate::layout::asked::{Asked, SizeAnswer};
use crate::layout::lane_page::VIEWPORT;
use crate::layout::tree::BoxTree;
use crate::layout::{LayoutBox, Written};

/// How every ask of every container of a kind came out.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct Compared {
    /// Containers every ask of which the mirror answered as Taffy did.
    pub(crate) containers: usize,
    /// Asks answered from the known dimensions alone.
    pub(crate) short: usize,
    /// Size queries whose recorded answer matched.
    pub(crate) size_asks: usize,
    /// Layouts whose whole answer and children's layouts matched.
    pub(crate) layout_asks: usize,
    /// Size queries with no recorded answer to compare against (the
    /// answer bubbled an out-of-flow box, or was disowned).
    pub(crate) unanswered: usize,
    /// Every ask declined, by reason, in ask order.
    pub(crate) declined: Vec<LaneDecline>,
}

/// How one ask came out when the mirror answered it.
pub(crate) enum Answered {
    Short,
    Size,
    Layout,
    Unanswered,
}

/// What a container's ask compared as: `compared` false when nothing
/// was recorded, `short` when answered from its known sizes.
pub(crate) fn answered(compared: bool, short: bool, run_mode: RunMode) -> Answered {
    match (compared, short, run_mode) {
        (false, _, _) => Answered::Unanswered,
        (true, true, _) => Answered::Short,
        (true, false, RunMode::PerformLayout) => Answered::Layout,
        (true, false, _) => Answered::Size,
    }
}

/// Every container in `doc` that `is_container` names, every ask of it
/// compared by `compare_ask`.
pub(crate) fn compare_each(
    doc: &mut Document,
    is_container: fn(&LayoutBox) -> bool,
    compare_ask: fn(&mut BoxTree<'_>, usize, &Asked) -> Result<Answered, LaneDecline>,
) -> Compared {
    let kept = doc.kept.as_ref().expect("updated");
    let placed = std::mem::take(&mut doc.boxes.placed);
    let mut lanes = placed.lanes.borrow_mut();
    let mut tree = BoxTree {
        boxes: placed.boxes,
        fonts: &mut doc.fonts,
        cache: &mut doc.texts,
        scale: VIEWPORT.scale,
        calcs: &kept.calcs,
        written: Written::default(),
        calls: 0,
        disowned: 0,
        lanes: &mut lanes,
        lanes_enabled: false,
        stale: rustc_hash::FxHashSet::default(),
        lane_counts: crate::layout::lane_layout::LaneCounts::default(),
    };
    let mut compared = Compared::default();
    for slot in 0..tree.boxes.len() {
        if is_container(&tree.boxes[slot]) {
            compare_container(&mut tree, slot, &mut compared, compare_ask);
        }
    }
    compared
}

/// Every ask of the container at `slot` compared and accounted for.
fn compare_container(
    tree: &mut BoxTree<'_>,
    slot: usize,
    compared: &mut Compared,
    compare_ask: fn(&mut BoxTree<'_>, usize, &Asked) -> Result<Answered, LaneDecline>,
) {
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

/// Taffy's answer to `ask`, from the container's cache.
pub(crate) fn taffy_answer(tree: &mut BoxTree<'_>, id: NodeId, ask: &Asked) -> LayoutOutput {
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
pub(crate) fn assert_answer(
    slot: usize,
    ask: &Asked,
    taffy: &LayoutOutput,
    mine: &LayoutOutput,
) -> bool {
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
