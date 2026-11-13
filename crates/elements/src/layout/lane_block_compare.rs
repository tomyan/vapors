//! How `lane_block_tests.rs` compares the mirror with Taffy: on a page
//! laid out by Taffy, each block container is asked by the mirror
//! every way its parent asked it, and the container's answer and each
//! child's layout are compared, exactly.

use taffy::{BlockFormattingContext, LayoutInput, LayoutOutput, NodeId, RunMode};
use vapors_dom::{Dom, NodeId as DomNode};
use vapors_text::FontSystem;

use crate::document::Document;
use crate::lane_decline::LaneDecline;
use crate::layout::asked::{Asked, SizeAnswer};
use crate::layout::lane_block::{Container, Settled};
use crate::layout::lane_block_answer::lane_answer;
use crate::layout::lane_block_summary::BlockSummary;
use crate::layout::lane_tree::Summary;
use crate::layout::tree::BoxTree;
use crate::layout::{Content, LayoutBox, Written};
use crate::update::Viewport;

const VIEWPORT: Viewport = Viewport {
    width: 300.0,
    height: 200.0,
    scale: 1.0,
};

/// A page styled by `css`, built under body by `build`, laid out once.
pub(super) fn page(css: &str, build: impl FnOnce(&mut Document, DomNode)) -> Document {
    let mut doc = Document::new(FontSystem::bundle_only());
    let body = doc.body();
    build(&mut doc, body);
    doc.add_stylesheet("page.css", css).expect("parses");
    doc.update(VIEWPORT).expect("updates");
    doc
}

/// `tag.class` under `parent`, with `text` in it when given.
pub(super) fn element(
    doc: &mut Document,
    parent: DomNode,
    class: &str,
    text: Option<&str>,
) -> DomNode {
    let node = doc.create_element("div");
    doc.set_attribute(node, "class", class);
    doc.append(parent, node);
    if let Some(text) = text {
        let text = doc.create_text(text);
        doc.append(node, text);
    }
    node
}

/// How many containers the mirror laid out as Taffy did, and how many
/// it declined, by reason.
#[derive(Debug, Default, PartialEq)]
pub(super) struct Compared {
    pub(super) containers: usize,
    pub(super) asks: usize,
    pub(super) declined: Vec<LaneDecline>,
}

/// Compares every block container with children in `doc` as laid out.
pub(super) fn compare(doc: &mut Document) -> Compared {
    let kept = doc.kept.as_ref().expect("updated");
    let placed = std::mem::take(&mut doc.boxes.placed);
    let mut tree = BoxTree {
        boxes: placed.boxes,
        fonts: &mut doc.fonts,
        cache: &mut doc.texts,
        scale: VIEWPORT.scale,
        calcs: &kept.calcs,
        written: Written::default(),
        calls: 0,
        disowned: 0,
    };
    let mut compared = Compared::default();
    for slot in 0..tree.boxes.len() {
        if !is_block_container(&tree.boxes[slot]) {
            continue;
        }
        let asked = tree.boxes[slot].asked.clone();
        let mut declined = None;
        for ask in &asked {
            match compare_ask(&mut tree, slot, ask) {
                Ok(()) => compared.asks += 1,
                Err(decline) => declined = Some(decline),
            }
        }
        match declined {
            Some(decline) => compared.declined.push(decline),
            None => compared.containers += 1,
        }
    }
    compared
}

fn is_block_container(layout_box: &LayoutBox) -> bool {
    matches!(layout_box.content, Content::Element(_))
        && layout_box.style.display == taffy::Display::Block
        && !layout_box.children.is_empty()
}

/// Asks the container at `slot` one way and compares; the decline
/// when the mirror does not lay it out.
fn compare_ask(tree: &mut BoxTree<'_>, slot: usize, ask: &Asked) -> Result<(), LaneDecline> {
    let id = NodeId::from(slot);
    let taffy = taffy_answer(tree, id, ask);
    let mine = match tree.lane_container(id, ask.input, ask.in_block) {
        Settled::Short(short) => short,
        Settled::Declined(decline) => return Err(decline),
        Settled::Laid(container) => {
            let children = tree.boxes[slot].children.clone();
            let mut run = BlockSummary::default();
            for child in children {
                let laid = tree.lane_child(NodeId::from(child), &container)?;
                let leaf = BlockSummary::leaf(laid.leaf)?;
                if ask.input.run_mode == RunMode::PerformLayout {
                    assert_child_layout(
                        tree,
                        (slot, child),
                        (&container, &run, &leaf),
                        laid.layout,
                    );
                }
                run = run.combine(&leaf, 0.0);
            }
            lane_answer(&container, &run)
        }
    };
    assert_answer(slot, ask, &taffy, &mine);
    Ok(())
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
fn assert_answer(slot: usize, ask: &Asked, taffy: &LayoutOutput, mine: &LayoutOutput) {
    let input: LayoutInput = ask.input;
    match input.run_mode {
        RunMode::PerformLayout => assert_eq!(taffy, mine, "slot {slot}, layout {input:?}"),
        RunMode::ComputeSize => {
            let Some(recorded) = ask.answered else {
                return;
            };
            assert_eq!(
                SizeAnswer::of(mine),
                Some(recorded),
                "slot {slot}, size {input:?}"
            );
        }
        RunMode::PerformHiddenLayout => unreachable!("a hidden layout is not asked of a lane"),
    }
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
    layout.location.y +=
        crate::layout::lane_block_answer::narrow(f64::from(container.resolved_inset.top) + start);
    let taffy = tree.boxes[child].layout;
    layout.order = taffy.order;
    assert_eq!(taffy, layout, "slot {slot}, child {child}");
}
