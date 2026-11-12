//! A focus move's sequence (3-E2a; review-fable-input M1), as Blink's
//! `Document::SetFocusedElement` orders it: the destination is held
//! aside, so during the leaving element's commit, `blur` and `focusout`
//! nothing has focus (`active_element` answers the body, as Chromium's
//! `activeElement` does); a handler that focused something meanwhile
//! keeps it, and the destination hears nothing; otherwise the
//! destination, if still focusable, takes focus with `focus` and
//! `focusin`, and a `focus` handler that moves focus on keeps that,
//! with no `focusin` to the first. The blur case is Chromium 155's log
//! (`tests/focus_handlers_move.rs`); the re-checks after `focus` and
//! for the destination's focusability are from reading Blink.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_css::tree::States;
use vapors_dom::{Event, NodeId};

use crate::document::Document;
use crate::events::tree::{dispatch_bubbling, dispatch_here};
use crate::focus_style::Focus;

impl Document {
    /// Focus leaves `had` for `node`, which is held aside: nothing has
    /// focus, and `:focus-within` stays on what holds both, as Blink
    /// keeps it up to their common ancestor.
    fn leave_focus(&mut self, had: Option<NodeId>, node: Option<NodeId>) {
        self.focused = None;
        self.set_flag_on(States::FOCUS, None);
        self.set_flag_on(States::FOCUS_VISIBLE, None);
        let shared = had.zip(node).and_then(|(a, b)| self.common_ancestor(a, b));
        self.set_flag_chain(States::FOCUS_WITHIN, shared);
    }

    /// Whether `node` can take focus after `had`'s handlers ran, which
    /// may have hidden or removed it; without any, `Document::focus`
    /// answered this just before.
    fn still_focusable(&self, had: Option<NodeId>, node: NodeId) -> bool {
        had.is_none() || self.focusability(node) == Focus::Done
    }
}

/// Focus moves from `had` to `node`, with the events.
pub(crate) fn move_focus(
    document: &Rc<RefCell<Document>>,
    had: Option<NodeId>,
    node: Option<NodeId>,
) {
    document.borrow_mut().leave_focus(had, node);
    // while the window is away focus moves silently; its return fires
    // `focus` on what has it then (`focus_window.rs`)
    if document.borrow().window_away() {
        document.borrow_mut().take_focus(node);
        return;
    }
    if let Some(had) = had {
        left(document, had);
    }
    // a `change` or `blur` handler that moved focus on keeps it, and
    // `node` gets no `focus`, as in Chromium
    if document.borrow().active_element().is_some() {
        return;
    }
    let still = node.filter(|node| document.borrow().still_focusable(had, *node));
    let Some(node) = still else {
        document
            .borrow_mut()
            .set_flag_chain(States::FOCUS_WITHIN, None);
        return;
    };
    document.borrow_mut().take_focus(Some(node));
    arrived(document, node);
}

/// Focus has left `had`: a composition ends, a number or text field
/// commits, with `change`, then `blur` and `focusout` fire, as Chromium
/// orders them with nothing focused (3-E3c, E6a).
fn left(document: &Rc<RefCell<Document>>, had: NodeId) {
    document.borrow_mut().end_composition(had);
    crate::number_input::commit(document, had);
    crate::text_input::commit(document, had);
    fire(document, had, &["blur", "focusout"]);
}

/// `node` took focus: `focus`, then `focusin` unless the `focus`
/// handler moved focus on or removed `node`, as Blink checks between
/// the two.
fn arrived(document: &Rc<RefCell<Document>>, node: NodeId) {
    dispatch_here(document, node, Event::new("focus"));
    if document.borrow().active_element() != Some(node) {
        return;
    }
    dispatch_bubbling(document, node, Event::new("focusin"));
}

/// `names` on `node` in order: the first only there (it does not bubble),
/// the second bubbling.
pub(crate) fn fire(document: &Rc<RefCell<Document>>, node: NodeId, names: &[&str; 2]) {
    dispatch_here(document, node, Event::new(names[0]));
    dispatch_bubbling(document, node, Event::new(names[1]));
}
