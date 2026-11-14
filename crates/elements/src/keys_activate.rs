//! A key's click on the focused control (`keys.rs`): Enter clicks a
//! button or a link, Space a button or a checkbox, as in Chromium, with
//! the key's modifiers on the click (a modified Enter opens a link
//! elsewhere, `link.rs`).

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::{Event, Modifiers, NodeId};

use crate::document::Document;

impl Document {
    /// A focused control's click with the key's `modifiers`, when it is
    /// one of `kinds` (`button` and `link` for Enter, `button` and
    /// `checkbox` for Space, as in Chromium) and not disabled; a checkbox
    /// then toggles, a link is followed where the modifiers say.
    pub(crate) fn activate(
        document: &Rc<RefCell<Document>>,
        target: NodeId,
        (kinds, modifiers): (&[&str], Modifiers),
    ) -> bool {
        if !document.borrow().activatable(target, kinds) {
            return false;
        }
        let click = Event::new("click").with_modifiers(modifiers);
        crate::events::tree::dispatch_click(document, target, click);
        true
    }

    /// Whether `target` is one of `kinds` (`button`, `link`, `checkbox`)
    /// and not disabled: a key would click it.
    pub(crate) fn activatable(&self, target: NodeId, kinds: &[&str]) -> bool {
        let kind = match self.tag(target) {
            Some("button") => "button",
            Some("a") if self.is_link(target) => "link",
            Some("input") if self.checked(target).is_some() => "checkbox",
            _ => return false,
        };
        kinds.contains(&kind) && !self.in_disabled_button(target)
    }
}
