//! The document: the node arena with `html` > `body` at its root, the
//! stylesheets, the errors the generated code's calls have caused, and
//! the result of the last update.

use vapors_dom::NodeId;
use vapors_text::FontSystem;

use crate::arena::Arena;
use crate::error::ElementError;
use crate::node::{Element, Kind, Node};
use crate::paint::Painted;
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
        }
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
    /// `name` is how errors refer to it.
    ///
    /// # Errors
    ///
    /// The CSS crate's refusal, with line and column; the sheet is not
    /// added.
    pub fn add_stylesheet(&mut self, name: &str, text: &str) -> Result<(), ElementError> {
        self.sheets.add_author(name, text)
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
