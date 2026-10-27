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
    /// The numbers of an `input type=range`, for paint (3-E3a).
    pub(crate) range: Option<crate::range::Range>,
    /// Whether an `input type=checkbox` is checked, for paint (3-E3b).
    pub(crate) checked: Option<bool>,
    /// A focused `input type=number`'s selection, for paint (3-E3c).
    pub(crate) number_focus: Option<NumberFocus>,
}

/// A focused number field: whether its text is selected whole.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NumberFocus {
    pub(crate) selected: bool,
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
            range: None,
            checked: None,
            number_focus: None,
        }
    }

    /// A box showing an image of `natural` size (CSS px), when it has
    /// one: a replaced box, which Taffy does not stretch, measured from its
    /// natural size and its limits (`replaced::size`). A checkbox has a
    /// natural size too but is not replaced: as a flex item it stretches
    /// across the line, as Chromium lays one out (`fixtures/checkbox`).
    pub(crate) fn replaced(
        content: Content,
        mut style: Style,
        (natural, image): (
            Option<[f32; 2]>,
            Option<std::rc::Rc<vapors_scene::RgbaImage>>,
        ),
        stretches: bool,
    ) -> Self {
        if natural.is_some() && !stretches {
            style.item_is_replaced = true;
        }
        Self {
            natural,
            image,
            ..Self::new(content, style)
        }
    }
}

/// What building reads: the tree, each element's used style, the
/// images, and the focused number field with its selection.
pub(crate) struct Source<'a> {
    pub(crate) arena: &'a Arena,
    pub(crate) used: &'a HashMap<NodeId, Used>,
    pub(crate) images: &'a crate::images::Images,
    pub(crate) number_focus: Option<(NodeId, NumberFocus)>,
}

/// Builds the boxes under `root`; box 0 is the root's.
pub(crate) fn build(source: &Source<'_>, root: NodeId) -> Vec<LayoutBox> {
    let mut boxes = Vec::new();
    add_element(source, root, &mut boxes);
    boxes
}

fn add_element(source: &Source<'_>, node: NodeId, boxes: &mut Vec<LayoutBox>) -> Option<usize> {
    let style = &source.used[&node];
    if style.layout.display == Display::None {
        return None;
    }
    let index = boxes.len();
    let range = crate::range::of_element(source.arena, node);
    let checked = source
        .arena
        .get(node, "build")
        .element()
        .and_then(crate::checkbox::checked_of);
    let natural = source
        .images
        .natural(source.arena, node)
        .or(range.map(|_| crate::range::NATURAL_SIZE))
        .or(checked.map(|_| crate::checkbox::NATURAL_SIZE));
    let mut replaced = LayoutBox::replaced(
        Content::Element(node),
        style.layout.clone(),
        (natural, source.images.of(source.arena, node).cloned()),
        checked.is_some(),
    );
    replaced.range = range;
    replaced.checked = checked;
    let field = source
        .arena
        .get(node, "build")
        .element()
        .filter(|e| e.tag == "input" && crate::check::is_type(e, "number"))
        .map(|e| crate::number::Field::of(&e.attributes, &e.properties));
    replaced.number_focus = source
        .number_focus
        .filter(|(focused, _)| *focused == node && field.is_some())
        .map(|(_, focus)| focus);
    boxes.push(replaced);
    let mut run = Collector::new(style.text, style.visible);
    // a number field's text is its run; a blank one keeps its line
    // with a no-break space, which paints nothing (3-E3c)
    if let Some(field) = field {
        run.push_text(if field.text.is_empty() {
            "\u{a0}"
        } else {
            &field.text
        });
    }
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
