//! The a11y mirror on the page (R6): `<div id="vapors-a11y">` appended
//! to `<body>`, fixed over the viewport, unpainted and out of
//! hit-testing (`opacity: 0` keeps its elements in every engine's
//! accessibility tree; `visibility: hidden` would take them out), with
//! the canvas `aria-hidden`, since it has no content of its own. The
//! mirror's elements are `<div>`s under it; a text field's value is a
//! leading text node, so a node's child elements stay.

use wasm_bindgen::JsCast;
use web_sys::{Element, Node};

use crate::PlatformError;
use crate::a11y_mirror_dom::{Mirror, MirrorDom};

/// The root's style, from r6.md: fixed over the viewport, nothing
/// painted, no pointer events.
const ROOT_STYLE: &str = "position:fixed;inset:0;overflow:hidden;pointer-events:none;opacity:0";

/// The page's DOM, for the mirror.
pub(crate) struct WebDom {
    document: web_sys::Document,
}

impl WebDom {
    /// The mirror under a new root on the page, the canvas hidden from
    /// readers.
    ///
    /// # Errors
    /// The page has no document, body or canvas, or refused the root.
    pub(crate) fn mirror() -> Result<(Self, Mirror<Self>), PlatformError> {
        let fail = |what: &str| PlatformError::Windowing(format!("the a11y mirror: {what}"));
        let document = web_sys::window()
            .and_then(|window| window.document())
            .ok_or_else(|| fail("no document"))?;
        let root = document
            .create_element("div")
            .map_err(|error| fail(&format!("{error:?}")))?;
        root.set_id("vapors-a11y");
        drop(root.set_attribute("style", ROOT_STYLE));
        document
            .body()
            .ok_or_else(|| fail("no body"))?
            .append_child(&root)
            .map_err(|error| fail(&format!("{error:?}")))?;
        drop(crate::web::canvas()?.set_attribute("aria-hidden", "true"));
        Ok((Self { document }, Mirror::new(root)))
    }
}

impl MirrorDom for WebDom {
    type Element = Element;

    fn create(&mut self) -> Result<Element, String> {
        self.document
            .create_element("div")
            .map_err(|error| format!("the page refused a mirror element: {error:?}"))
    }

    fn set_attribute(&mut self, element: &Element, name: &str, value: &str) {
        // the names are the mapping's constants, which the page accepts
        drop(element.set_attribute(name, value));
    }

    fn remove_attribute(&mut self, element: &Element, name: &str) {
        drop(element.remove_attribute(name));
    }

    fn set_text(&mut self, element: &Element, text: &str) {
        let first = element.first_child();
        if let Some(node) = first
            .as_ref()
            .filter(|node| node.node_type() == Node::TEXT_NODE)
        {
            node.set_text_content(Some(text));
        } else if !text.is_empty() {
            let node = self.document.create_text_node(text);
            drop(element.insert_before(node.unchecked_ref(), first.as_ref()));
        }
    }

    fn append(&mut self, parent: &Element, child: &Element) {
        drop(parent.append_child(child));
    }

    fn insert_before(&mut self, parent: &Element, child: &Element, before: &Element) {
        drop(parent.insert_before(child, Some(before)));
    }

    fn remove(&mut self, element: &Element) {
        element.remove();
    }
}
