//! The document: the node arena with `html` > `body` at its root, the
//! stylesheets, the errors the generated code's calls have caused, and
//! the result of the last update.

use vapors_dom::NodeId;
use vapors_text::FontSystem;

use crate::arena::Arena;
use crate::error::ElementError;
use crate::node::{Element, Kind, Node};
use crate::painted::Painted;
use crate::style::Sheets;

/// The element tree a component mounts into. Share it as
/// `Rc<RefCell<Document>>`, which is what generated code takes.
pub struct Document {
    pub(crate) arena: Arena,
    root: NodeId,
    body: NodeId,
    pub(crate) sheets: Sheets,
    pub(crate) fonts: FontSystem,
    pub(crate) painted: Option<Painted>,
    /// Whether anything shown may have changed (`update::Changes`).
    pub(crate) changes: crate::update::Changes,
    pub(crate) pointer: crate::events::Pointer,
    /// The focused element (3-E2a), `None` when the body has focus; and
    /// what the next `settle_focus` does (`focus_fixup`).
    pub(crate) focused: Option<NodeId>,
    pub(crate) settle: crate::focus_fixup::Settle,
    /// Whether the window has focus (`focus_window.rs`).
    pub(crate) window: crate::focus_window::Window,
    /// Where Space went down, for its click on keyup.
    pub(crate) space_on: Option<NodeId>,
    /// The link a followed click asks to open (`link.rs`).
    pub(crate) navigation: Option<crate::link::Navigation>,
    /// The modifier shortcuts take (`shortcuts`).
    pub(crate) shortcut_modifier: crate::ShortcutModifier,
    /// Each number field's last committed number and value (`number_committed`).
    pub(crate) committed: std::collections::HashMap<NodeId, crate::number_committed::Committed>,
    /// Each text field's editor (E6a).
    pub(crate) editors: crate::text_field::Editors,
    /// The events components dispatch themselves (`declare_events`),
    /// whose listeners the tree takes though it never fires them.
    pub(crate) declared_events: std::collections::BTreeSet<String>,
    /// Whether the last input was a key rather than a press (3-E2b):
    /// focus that arrives then matches `:focus-visible`.
    pub(crate) keyboard: bool,
    /// Parley layouts kept between updates.
    pub(crate) texts: crate::layout::TextCache,
    /// What changed since the last update, for restyle.
    pub(crate) dirty: crate::restyle::Dirty,
    /// The styles kept from the last update.
    pub(crate) kept: Option<crate::restyle::Kept>,
    /// The boxes the last update laid out, for the next to reuse.
    pub(crate) boxes: crate::layout::KeptBoxes,
    /// The style phase's checks, kept between updates.
    pub(crate) checks: crate::kept_checks::KeptChecks,
    /// The last update's structural counts.
    pub(crate) counts: crate::UpdateCounts,
    /// Each scroll container's offset.
    pub(crate) scroll: crate::scroll::Offsets,
    /// How opaque scroll thumbs are drawn now, 0 (hidden) to 255.
    pub(crate) thumb_alpha: u8,
    /// The images `img` elements show, by name.
    pub(crate) images: crate::images::Images,
    /// What the a11y mirror keeps between takes (R6).
    pub(crate) ax: crate::a11y_mirror_state::AxMirror,
    /// Each watched mesh anchor's appearance sink (`scene_sinks`).
    pub(crate) sinks: crate::node_map::NodeMap<vapors_dom::AppearanceSink>,
    /// Each watched scene anchor's hit source (`scene_hits`).
    pub(crate) scene_hits: crate::scene_hits::SceneHits,
    /// The run-time shapes its items name and which elements use them
    /// (one-scene S4a), shared with each `Painted` so the painter takes
    /// the slots that changed. Per document: S5b's process registry
    /// issues table `ShapeId`s, and this run-time range stays apart.
    pub(crate) shapes: std::rc::Rc<std::cell::RefCell<crate::document_shapes::DocumentShapes>>,
}

impl Document {
    /// An empty document, `html` > `body`, with the user-agent sheet,
    /// laying text out with `fonts`.
    #[must_use]
    pub fn new(fonts: FontSystem) -> Self {
        let mut arena = Arena::default();
        let root = arena.insert(Node::new(Kind::Element(Element::new("html"))));
        let body = arena.insert(Node::new(Kind::Element(Element::new("body"))));
        arena.place("new", root, body, None);
        Self {
            arena,
            root,
            body,
            sheets: Sheets::new(),
            fonts,
            painted: None,
            changes: crate::update::Changes::default(),
            pointer: crate::events::Pointer::default(),
            focused: None,
            settle: crate::focus_fixup::Settle::default(),
            window: crate::focus_window::Window::Focused,
            space_on: None,
            navigation: None,
            shortcut_modifier: crate::ShortcutModifier::default(),
            committed: std::collections::HashMap::new(),
            editors: crate::text_field::Editors::new(),
            declared_events: std::collections::BTreeSet::new(),
            keyboard: false,
            texts: crate::layout::TextCache::default(),
            dirty: crate::restyle::Dirty::default(),
            kept: None,
            boxes: crate::layout::KeptBoxes::default(),
            checks: crate::kept_checks::KeptChecks::default(),
            counts: crate::UpdateCounts::default(),
            scroll: crate::scroll::Offsets::default(),
            thumb_alpha: u8::MAX,
            images: crate::images::Images::default(),
            ax: crate::a11y_mirror_state::AxMirror::default(),
            sinks: crate::node_map::NodeMap::default(),
            scene_hits: crate::scene_hits::SceneHits::default(),
            shapes: std::rc::Rc::new(std::cell::RefCell::new(
                crate::document_shapes::DocumentShapes::new(),
            )),
        }
    }

    /// Whether anything that would change the next update happened since
    /// the last call: the tree, an attribute, a text, a stylesheet, the
    /// pointer's hover and active state, or a scroll. An app that redraws on change
    /// asks this after each event. True for a new document.
    pub fn take_changed(&mut self) -> bool {
        // a refused scene hit is due in the next update (`scene_hit`)
        std::mem::replace(&mut self.changes.since_asked, false) || self.hit_refusals_pending()
    }

    /// The `html` element.
    #[must_use]
    pub fn root(&self) -> NodeId {
        self.root
    }

    /// The `body` element, where components usually mount.
    #[must_use]
    pub fn body(&self) -> NodeId {
        self.body
    }

    /// Adds a global author stylesheet, after those already added.
    /// `name` is how errors refer to it. Refused once an analysed
    /// element (one made from a key with compiled winners) exists, and
    /// such an element is refused in a document holding one
    /// (build-time-styles.md, decision 2).
    ///
    /// # Errors
    ///
    /// An analysed element exists, naming it and the fix; or the CSS
    /// crate's refusal, with line and column. The sheet is not added.
    pub fn add_stylesheet(&mut self, name: &str, text: &str) -> Result<(), ElementError> {
        // a refused call changes nothing (`dom_keyed.rs`)
        self.sheets.add_rust(name, text)?;
        self.note_change();
        self.dirty.everything();
        Ok(())
    }

    /// Whether `node` is in the document (created and not removed).
    #[must_use]
    pub fn contains(&self, node: NodeId) -> bool {
        self.arena.contains(node)
    }

    /// The element's tag, or `None` for a text node.
    #[must_use]
    pub fn tag(&self, node: NodeId) -> Option<&str> {
        self.arena
            .get(node, "tag")
            .element()
            .map(|e| e.tag.as_str())
    }

    /// An element's attribute, or `None` when it has none of that name
    /// (or `node` is a text node).
    #[must_use]
    pub fn attribute(&self, node: NodeId, name: &str) -> Option<&str> {
        self.arena
            .get(node, "attribute")
            .element()?
            .attributes
            .get(name)
            .map(String::as_str)
    }

    /// The text of a text node, or `None` for an element.
    #[must_use]
    pub fn text(&self, node: NodeId) -> Option<&str> {
        match &self.arena.get(node, "text").kind {
            Kind::Text(text) => Some(text),
            Kind::Element(_) => None,
        }
    }

    /// The children of `node`, in order.
    #[must_use]
    pub fn children(&self, node: NodeId) -> Vec<NodeId> {
        self.arena.children(node)
    }

    /// The parent of `node`, or `None` for the root and detached nodes.
    #[must_use]
    pub fn parent(&self, node: NodeId) -> Option<NodeId> {
        self.arena.get(node, "parent").parent
    }

    /// Every element under the root, the root first, in document order.
    #[must_use]
    pub fn elements(&self) -> Vec<NodeId> {
        crate::check::elements(&self.arena, self.root)
    }
}
