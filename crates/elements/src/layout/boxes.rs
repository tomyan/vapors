//! The box tree: what Taffy lays out, derived from the document on each
//! update. An element with `display: none` has no box. Each run of a
//! block's consecutive inline content (text nodes and `display: inline`
//! elements) becomes one anonymous box holding an [`InlineRun`](super::inline::InlineRun); a run
//! that collapses to nothing has no box (the empty anchors `{if}` and
//! `{for}` leave, and the whitespace between blocks).

use crate::node_map::NodeMap;

use rustc_hash::FxHashMap;
use taffy::{Display, Style};
use vapors_dom::NodeId;

use super::inline::{Collector, InlineSpan};
pub(crate) use super::layout_box::{Content, LayoutBox};
use super::owners::KeptChildren;
use crate::arena::Arena;
use crate::node::Kind;
use crate::style::Used;

/// What building reads: the tree, each element's used style, the
/// images, and the focused number field with its selection.
pub(crate) struct Source<'a> {
    pub(crate) arena: &'a Arena,
    pub(crate) used: &'a NodeMap<Used>,
    pub(crate) images: &'a crate::images::Images,
    /// The focused text or number field's marks (E6a, 3-E3c).
    pub(crate) text_marks: Option<(NodeId, crate::text_marks::TextMarks)>,
}

/// Builds the boxes under `root`; box 0 is the root's.
pub(crate) fn build(source: &Source<'_>, root: NodeId) -> Vec<LayoutBox> {
    let mut boxes = Vec::new();
    add_element(source, root, &mut boxes, None);
    boxes
}

/// Builds `owner`'s box and its children's, with a placeholder for
/// each `kept` child in place of its subtree; the boxes, and each
/// placeholder's index with the kept child's slot.
pub(crate) fn build_shallow(
    source: &Source<'_>,
    owner: NodeId,
    kept: &KeptChildren,
) -> (Vec<LayoutBox>, FxHashMap<usize, usize>) {
    let mut boxes = Vec::new();
    add_element(source, owner, &mut boxes, Some(kept));
    let placeholder = |&index: &usize| match boxes[index].content {
        Content::Element(node) => Some((index, *kept.get(&node)?)),
        _ => None,
    };
    let placeholders = boxes
        .first()
        .map(|owner| owner.children.iter().filter_map(placeholder));
    let placeholders = placeholders.into_iter().flatten().collect();
    (boxes, placeholders)
}

fn add_element(
    source: &Source<'_>,
    node: NodeId,
    boxes: &mut Vec<LayoutBox>,
    kept: Option<&KeptChildren>,
) -> Option<usize> {
    let style = &source.used[&node];
    if style.layout.display == Display::None {
        return None;
    }
    let index = boxes.len();
    let (own, field_text) = element_box(source, node);
    boxes.push(own);
    let mut run = Collector::new(style.text, style.visible);
    // a number or text field's text is its run; a blank one keeps its
    // line with a no-break space, which paints nothing (3-E3c, E6a)
    if let Some(text) = field_text {
        run.push_text(if text.is_empty() { "\u{a0}" } else { &text });
    }
    for child in box_children(source.arena, node) {
        match &source.arena.get(child, "build").kind {
            Kind::Text(text) => run.push_text(text),
            // No box, and no break in the run around it (CSS Display 3,
            // 2.5: `display: none` generates nothing).
            Kind::Element(_) if source.used[&child].layout.display == Display::None => {}
            Kind::Element(_) if source.used[&child].inline => add_inline(source, child, &mut run),
            Kind::Element(_) => {
                let ended = std::mem::replace(&mut run, Collector::new(style.text, style.visible));
                add_run(ended, index, boxes);
                let child_box = match kept {
                    Some(kept) if kept.contains_key(&child) => Some(placeholder(child, boxes)),
                    _ => add_element(source, child, boxes, None),
                };
                if let Some(child_box) = child_box {
                    boxes[index].children.push(child_box);
                }
            }
        }
    }
    add_run(run, index, boxes);
    Some(index)
}

/// `node`'s own box, with no children, and the text of a text or
/// number field, which is its run.
pub(super) fn element_box(source: &Source<'_>, node: NodeId) -> (LayoutBox, Option<String>) {
    let style = &source.used[&node];
    let range = crate::range::of_element(source.arena, node);
    let checked = source
        .arena
        .get(node, "build")
        .element()
        .and_then(crate::checkbox::checked_of);
    let natural = source
        .images
        .natural(source.arena, node)
        .or_else(|| crate::svg::natural(source.arena, node))
        .or(range.map(|_| crate::range::NATURAL_SIZE))
        .or(checked.map(|_| crate::checkbox::NATURAL_SIZE));
    let mut own = LayoutBox::replaced(
        Content::Element(node),
        crate::svg::layout_style(source.arena, node, &style.layout),
        (natural, source.images.of(source.arena, node).cloned()),
        checked.is_some(),
    );
    own.icon = crate::svg_paint::icon(source.arena, node);
    own.surface = crate::surfaces::name(source.arena, node).map(Into::into);
    own.range = range;
    own.checked = checked;
    let field_text = super::field_box::fill(source, node, &mut own);
    (own, field_text)
}

/// A placeholder for kept `child`'s box; its index.
fn placeholder(child: NodeId, boxes: &mut Vec<LayoutBox>) -> usize {
    boxes.push(LayoutBox::new(Content::Element(child), Style::default()));
    boxes.len() - 1
}

/// The children of `node` that may have boxes: none for an `svg`,
/// whose paths are drawn as its content (docs/icons.md, Layout).
fn box_children(arena: &Arena, node: NodeId) -> Vec<NodeId> {
    if crate::svg::is(arena, node, "svg") {
        return Vec::new();
    }
    arena.children(node)
}

/// An inline element's text into `run`, its inline descendants' too.
/// A block inside one is refused when styles are read.
fn add_inline(source: &Source<'_>, node: NodeId, run: &mut Collector) {
    let style = &source.used[&node];
    if style.layout.display == Display::None {
        return;
    }
    let (slot, start) = (run.open(), run.position());
    for child in source.arena.children(node) {
        match &source.arena.get(child, "build").kind {
            Kind::Text(text) => run.push_text(text),
            Kind::Element(_) => add_inline(source, child, run),
        }
    }
    let span = InlineSpan {
        node,
        range: start..run.position(),
        style: style.text,
        visible: style.visible,
    };
    run.close(slot, span);
}

fn add_run(run: Collector, parent: usize, boxes: &mut Vec<LayoutBox>) {
    let Some(run) = run.finish() else {
        return;
    };
    let index = boxes.len();
    let box_style = Style {
        display: Display::Block,
        ..Style::default()
    };
    boxes.push(LayoutBox::new(Content::Text(run), box_style));
    boxes[parent].children.push(index);
}
