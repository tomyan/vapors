//! Focus (3-E2a): one focused element per document. Focusable are
//! `button`, `input`, `textarea`, `select`, `summary`, `a` with `href`
//! and any element with `tabindex` 0 or -1, unless disabled or without a
//! box; Tab and Shift+Tab move through those whose tabindex is not -1,
//! in tree order, and past the last focus leaves the page (nothing has
//! focus), as in Chromium. A move fires `blur` and `focusout` on the
//! element losing focus, then `focus` and `focusin` on the one gaining
//! it, as Chromium orders them; `:focus` is set on the element and
//! `:focus-within` on it and its ancestors.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_css::tree::States;
use vapors_dom::{Event, NodeId};

use crate::document::Document;

/// Focusable without a `tabindex`.
const INTERACTIVE: [&str; 5] = ["button", "input", "textarea", "select", "summary"];

impl Document {
    /// The focused element, or `None` when the body has focus.
    #[must_use]
    pub fn active_element(&self) -> Option<NodeId> {
        self.focused.filter(|node| self.contains(*node))
    }

    /// Whether the focused element matches `:focus-visible`: focus that
    /// arrived after a key (by Tab, or by script after a key), or on a
    /// text field, as Chromium's heuristic has it; not after a press.
    #[must_use]
    pub fn focus_visible(&self) -> bool {
        self.active_element().is_some_and(|node| {
            self.arena
                .get(node, "focus_visible")
                .element()
                .is_some_and(|element| element.states.contains(States::FOCUS_VISIBLE))
        })
    }

    fn is_text_field(&self, node: NodeId) -> bool {
        match self.tag(node) {
            Some("textarea") => true,
            Some("input") => matches!(
                self.attribute(node, "type"),
                None | Some("text" | "search" | "email" | "url" | "password" | "tel" | "number")
            ),
            _ => false,
        }
    }

    /// Moves focus to `node` (`None`: to nothing), firing the focus
    /// events; nothing happens when it already has focus.
    pub fn focus(document: &Rc<RefCell<Document>>, node: Option<NodeId>) {
        let had = document.borrow().active_element();
        if had == node {
            return;
        }
        {
            let mut doc = document.borrow_mut();
            doc.focused = node;
            doc.set_flag_on(States::FOCUS, node);
            let visible = node.filter(|node| doc.keyboard || doc.is_text_field(*node));
            doc.set_flag_on(States::FOCUS_VISIBLE, visible);
            doc.set_flag_chain(States::FOCUS_WITHIN, node);
        }
        if let Some(had) = had {
            fire(document, had, &["blur", "focusout"]);
        }
        if let Some(node) = node {
            fire(document, node, &["focus", "focusin"]);
        }
    }

    /// The element Tab (or, `back`, Shift+Tab) moves focus to: the next
    /// in tree order that Tab visits; past the last, `None` (focus leaves
    /// the page, as in Chromium: only a focus trap wraps), and from
    /// `None` the first.
    pub(crate) fn next_in_tab_order(&self, back: bool) -> Option<NodeId> {
        // every element in tree order, so a focused element outside the
        // Tab order (tabindex -1) still has a place to go on from
        let mut all = self.elements();
        if back {
            all.reverse();
        }
        let from = self
            .active_element()
            .and_then(|node| all.iter().position(|n| *n == node))
            .map_or(0, |at| at + 1);
        all[from.min(all.len())..]
            .iter()
            .copied()
            .find(|node| self.tabbable(*node))
    }

    /// When `removed` is, or holds, the focused element: nothing has
    /// focus, and `:focus` and `:focus-within` clear (review-v81). No
    /// `blur` fires, as Chromium sends none for a removed element.
    pub(crate) fn drop_focus_within(&mut self, removed: NodeId) {
        let Some(focused) = self.active_element() else {
            return;
        };
        if self.chain(focused).contains(&removed) {
            self.focused = None;
            self.set_flag_on(States::FOCUS, None);
            self.set_flag_chain(States::FOCUS_WITHIN, None);
            self.set_flag_on(States::FOCUS_VISIBLE, None);
        }
    }

    /// The nearest element holding `node` (or `node`) that a press
    /// focuses: any focusable one, `tabindex=-1` included.
    pub(crate) fn focusable_holding(&self, node: NodeId) -> Option<NodeId> {
        self.chain(node).into_iter().find(|n| self.focusable(*n))
    }

    fn tabbable(&self, node: NodeId) -> bool {
        self.focusable(node) && self.attribute(node, "tabindex") != Some("-1")
    }

    fn focusable(&self, node: NodeId) -> bool {
        let Some(tag) = self.tag(node) else {
            return false;
        };
        let named =
            INTERACTIVE.contains(&tag) || (tag == "a" && self.attribute(node, "href").is_some());
        let indexed = matches!(self.attribute(node, "tabindex"), Some("0" | "-1"));
        let drawn = self
            .painted
            .as_ref()
            .is_some_and(|painted| painted.is_visible(node));
        (named || indexed) && drawn && !self.in_disabled_control(node)
    }

    fn in_disabled_control(&self, node: NodeId) -> bool {
        self.tag(node).is_some_and(|tag| INTERACTIVE.contains(&tag))
            && self.attribute(node, "disabled").is_some()
    }
}

/// `names` on `node` in order: the first only there (it does not bubble),
/// the second bubbling.
fn fire(document: &Rc<RefCell<Document>>, node: NodeId, names: &[&str; 2]) {
    crate::events::tree::dispatch_here(document, node, Event::new(names[0]));
    crate::events::tree::dispatch_bubbling(document, node, Event::new(names[1]));
}

/// A `tabindex` the tree refuses: a positive one (it orders focus by
/// number, which the WAI-ARIA Authoring Practices advise against) or one
/// that is not a whole number.
pub(crate) fn tabindex_refusal(value: Option<&String>) -> Option<String> {
    let value = value?;
    match value.trim().parse::<i32>() {
        Ok(0 | -1) => None,
        Ok(_) | Err(_) => Some(format!(
            "tabindex=\"{value}\": a tabindex is 0 (in the Tab order) or -1 (focusable by a press or by script only); a positive one, which orders focus by number, is refused"
        )),
    }
}
