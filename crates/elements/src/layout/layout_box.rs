//! A box: what it is, its Taffy style and children, and what layout and
//! paint keep on it.

use taffy::{Cache, Layout, Style};
use vapors_dom::NodeId;

use super::inline::InlineRun;

/// What a box is.
#[derive(PartialEq)]
pub(crate) enum Content {
    /// An element's box; its children are boxes too.
    Element(NodeId),
    /// An anonymous box holding an inline run.
    Text(InlineRun),
    /// A slot whose box is gone (`slots`); no box's child.
    Vacant,
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
    /// The icon an `svg` draws, for paint.
    pub(crate) icon: Option<std::rc::Rc<vapors_text::Icon>>,
    /// The name of the renderer a `gpu-surface` shows, for paint.
    pub(crate) surface: Option<std::rc::Rc<str>>,
    /// The numbers of an `input type=range`, for paint (3-E3a).
    pub(crate) range: Option<crate::range::Range>,
    /// Whether an `input type=checkbox` is checked, for paint (3-E3b).
    pub(crate) checked: Option<bool>,
    /// Whether the element is a text or number field (E6a, 3-E3c),
    /// whose text origin paint keeps for the pointer.
    pub(crate) text_field: bool,
    /// A focused text field's caret and selection, for paint.
    pub(crate) text_marks: Option<crate::text_marks::TextMarks>,
    /// Whether the box was kept from the last update rather than built
    /// in this one: only a kept box's kept paint may be copied
    /// (`kept_box.rs`).
    pub(crate) kept: bool,
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
            icon: None,
            surface: None,
            range: None,
            checked: None,
            text_field: false,
            text_marks: None,
            kept: false,
        }
    }

    /// The element of the box in `slot`, this box; none for a text box.
    /// Panics on a vacant slot, which no walk from the root reaches.
    pub(crate) fn element_in(&self, slot: usize) -> Option<NodeId> {
        match self.content {
            Content::Element(node) => Some(node),
            Content::Text(_) => None,
            Content::Vacant => unreachable!("slot {slot} is vacant, and no box's child"),
        }
    }

    /// What a vacant slot holds.
    pub(crate) fn vacant() -> Self {
        Self::new(Content::Vacant, Style::default())
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
