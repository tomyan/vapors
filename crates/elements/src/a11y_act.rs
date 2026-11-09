//! Acting on one node of the a11y tree (R1), found by role and name
//! (`a11y_find`), through a person's input path, so focus and the
//! events happen as they do for a person: typing is each character's
//! key going down, its text, and the key coming up, as the platform
//! sends them, to the node focused first (a press: `a11y_press`). A
//! disabled node is refused, and so is a key once focus has left the
//! node: a listener may move it mid-way.

use std::cell::RefCell;
use std::rc::Rc;

use crate::a11y_error::AxError;
use crate::a11y_target::AxTarget;
use crate::a11y_tree::AxElement;
use crate::document::Document;
use crate::keys::Modifiers;

impl Document {
    /// Focuses the node `target` names, as its `focus()` does; nothing
    /// happens when it has focus already.
    ///
    /// # Errors
    /// As [`Document::ax_find`]; [`AxError::Disabled`];
    /// [`AxError::NotFocusable`].
    pub fn ax_focus(document: &Rc<RefCell<Document>>, target: &AxTarget) -> Result<(), AxError> {
        Self::ax_focused(document, target).map(|_| ())
    }

    /// Types `text` into the node `target` names, focusing it first:
    /// each character's key goes down, its text arrives, its key comes
    /// up, while the node keeps focus.
    ///
    /// # Errors
    /// As [`Document::ax_focus`]; [`AxError::FocusLost`] when focus
    /// leaves the node before a key.
    pub fn ax_type(
        document: &Rc<RefCell<Document>>,
        target: &AxTarget,
        text: &str,
    ) -> Result<(), AxError> {
        let found = Self::ax_focused(document, target)?;
        let keys: Vec<String> = text.chars().map(String::from).collect();
        for (sent, key) in keys.iter().enumerate() {
            document.borrow().ax_keeps_focus(&found, sent, keys.len())?;
            Self::key_pressed(document, key, Modifiers::default());
            Self::character_typed(document, key);
            Self::key_released(document, key, Modifiers::default());
        }
        Ok(())
    }

    /// Presses `key` with `modifiers` on the node `target` names,
    /// focusing it first: the key goes down and comes up.
    ///
    /// # Errors
    /// As [`Document::ax_type`].
    pub fn ax_key(
        document: &Rc<RefCell<Document>>,
        target: &AxTarget,
        key: &str,
        modifiers: Modifiers,
    ) -> Result<(), AxError> {
        let found = Self::ax_focused(document, target)?;
        document.borrow().ax_keeps_focus(&found, 0, 1)?;
        Self::key_pressed(document, key, modifiers);
        Self::key_released(document, key, modifiers);
        Ok(())
    }

    /// The node `target` names, when it is not disabled.
    pub(crate) fn ax_live(
        document: &Rc<RefCell<Document>>,
        target: &AxTarget,
    ) -> Result<AxElement, AxError> {
        let found = document
            .borrow()
            .ax_find(&target.role, &target.name, target.within())?;
        if found.states.disabled {
            return Err(AxError::Disabled {
                role: found.role,
                name: found.name,
            });
        }
        Ok(found)
    }

    /// The node `target` names, focused.
    fn ax_focused(
        document: &Rc<RefCell<Document>>,
        target: &AxTarget,
    ) -> Result<AxElement, AxError> {
        let found = Self::ax_live(document, target)?;
        if !document.borrow().focusable(found.node) {
            return Err(AxError::NotFocusable {
                role: found.role,
                name: found.name,
            });
        }
        Self::focus(document, Some(found.node));
        Ok(found)
    }

    /// Refuses when `found` has lost focus, `sent` of `of` keys in.
    fn ax_keeps_focus(&self, found: &AxElement, sent: usize, of: usize) -> Result<(), AxError> {
        let active = self.active_element();
        if active == Some(found.node) {
            return Ok(());
        }
        Err(AxError::FocusLost {
            role: found.role.clone(),
            name: found.name.clone(),
            to: active.map(|node| self.ax_label(node)),
            sent,
            of,
        })
    }

    /// `node` as a refusal names it: `role "name"`, or `<tag>` when it
    /// has no role.
    fn ax_label(&self, node: vapors_dom::NodeId) -> String {
        let ids = self.ax_ids();
        match self.ax_role(node, &ids) {
            Some(role) => format!("{role} \"{}\"", self.ax_name(node, &role, &ids)),
            None => format!("<{}>", self.tag(node).unwrap_or("?")),
        }
    }
}
