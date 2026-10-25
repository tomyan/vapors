//! The box tree: what Taffy lays out, derived from the document on each
//! update. An element with `display: none` has no box. Each run of a
//! block's consecutive inline content (text nodes and `display: inline`
//! elements) becomes one anonymous box holding an [`InlineRun`]; a run
//! that collapses to nothing has no box (the empty anchors `{if}` and
//! `{for}` leave, and the whitespace between blocks).

use std::collections::HashMap;

use taffy::{Cache, Display, Layout, Style};
use vapors_dom::NodeId;

use super::inline::{Collector, InlineRun, InlineSpan};
use crate::arena::Arena;
use crate::node::Kind;
use crate::style::Used;

/// What a box is.
#[derive(PartialEq)]
pub(crate) enum Content {
    /// An element's box; its children are boxes too.
    Element(NodeId),
    /// An anonymous box holding an inline run.
    Text(InlineRun),
}

/// One box: its content, Taffy style, children, and Taffy's results.
pub(crate) struct LayoutBox {
    pub(crate) content: Content,
    pub(crate) style: Style,
    pub(crate) children: Vec<usize>,
    pub(crate) cache: Cache,
    pub(crate) layout: Layout,
    /// The out-of-flow (absolute and fixed) boxes this box is the
    /// containing block of, as Taffy hoists them; their layouts are
    /// relative to this box.
    pub(crate) hoisted: Vec<usize>,
    /// The natural size of the image an `img` shows, CSS px; a kept
    /// layout is lent only to a box with the same.
    pub(crate) natural: Option<[f32; 2]>,
    /// The image an `img` shows, for paint.
    pub(crate) image: Option<std::rc::Rc<vapors_scene::RgbaImage>>,
}

impl LayoutBox {
    pub(crate) fn new(content: Content, style: Style) -> Self {
        Self {
            content,
            style,
            children: Vec::new(),
            cache: Cache::new(),
            layout: Layout::with_order(0),
            hoisted: Vec::new(),
            natural: None,
            image: None,
        }
    }

    /// A box showing an image of `natural` size (CSS px), when it has
    /// one: a replaced box, which Taffy does not stretch, measured from its
    /// natural size and its limits (`replaced::size`).
    pub(crate) fn replaced(
        content: Content,
        mut style: Style,
        (natural, image): (
            Option<[f32; 2]>,
            Option<std::rc::Rc<vapors_scene::RgbaImage>>,
        ),
    ) -> Self {
        if natural.is_some() {
            style.item_is_replaced = true;
        }
        Self {
            natural,
            image,
            ..Self::new(content, style)
        }
    }
}

/// What building reads: the tree and each element's used style.
struct Source<'a> {
    arena: &'a Arena,
    used: &'a HashMap<NodeId, Used>,
    images: &'a crate::images::Images,
}

/// Builds the boxes under `root`; box 0 is the root's.
pub(crate) fn build(
    arena: &Arena,
    used: &HashMap<NodeId, Used>,
    images: &crate::images::Images,
    root: NodeId,
) -> Vec<LayoutBox> {
    let mut boxes = Vec::new();
    add_element(
        &Source {
            arena,
            used,
            images,
        },
        root,
        &mut boxes,
    );
    boxes
}

fn add_element(source: &Source<'_>, node: NodeId, boxes: &mut Vec<LayoutBox>) -> Option<usize> {
    let style = &source.used[&node];
    if style.layout.display == Display::None {
        return None;
    }
    let index = boxes.len();
    boxes.push(LayoutBox::replaced(
        Content::Element(node),
        style.layout.clone(),
        (
            source.images.natural(source.arena, node),
            source.images.of(source.arena, node).cloned(),
        ),
    ));
    let mut run = Collector::new(style.text, style.visible);
    for child in source.arena.children(node) {
        match &source.arena.get(child, "build").kind {
            Kind::Text(text) => run.push_text(text),
            // No box, and no break in the run around it (CSS Display 3,
            // 2.5: `display: none` generates nothing).
            Kind::Element(_) if source.used[&child].layout.display == Display::None => {}
            Kind::Element(_) if source.used[&child].inline => add_inline(source, child, &mut run),
            Kind::Element(_) => {
                let ended = std::mem::replace(&mut run, Collector::new(style.text, style.visible));
                add_run(ended, index, boxes);
                if let Some(child_box) = add_element(source, child, boxes) {
                    boxes[index].children.push(child_box);
                }
            }
        }
    }
    add_run(run, index, boxes);
    Some(index)
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
