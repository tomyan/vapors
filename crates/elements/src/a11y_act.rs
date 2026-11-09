//! Acting on one node of the a11y tree (R1), found by role and name
//! (`a11y_find`), through a person's input path, so focus, `:hover`,
//! `:active` and the events happen as they do for a person: a press is
//! the pointer moving to the node's centre, going down and coming up
//! there; typing is each character's key going down, its text, and the
//! key coming up, as the platform sends them. A disabled node is
//! refused, and so is a press whose point another element covers.

use std::cell::RefCell;
use std::rc::Rc;

use crate::a11y_error::AxError;
use crate::a11y_target::AxTarget;
use crate::a11y_tree::AxElement;
use crate::document::Document;
use crate::keys::Modifiers;
use crate::layout::BoxRect;

impl Document {
    /// Presses the node `target` names: the pointer moves to its centre,
    /// goes down and comes up there.
    ///
    /// # Errors
    /// As [`Document::ax_find`]; [`AxError::Disabled`];
    /// [`AxError::Covered`] when the centre reaches another element.
    pub fn ax_press(document: &Rc<RefCell<Document>>, target: &AxTarget) -> Result<(), AxError> {
        let found = Self::ax_live(document, target)?;
        let at = centre(found.bounds);
        let hit = document.borrow().hit(at);
        let doc = document.borrow();
        if !hit.is_some_and(|hit| doc.chain(hit).contains(&found.node)) {
            let by = hit.and_then(|hit| doc.tag(hit)).map(str::to_owned);
            return Err(AxError::Covered {
                role: found.role,
                name: found.name,
                at,
                by,
            });
        }
        drop(doc);
        Self::pointer_moved(document, at);
        Self::pointer_pressed(document, at);
        Self::pointer_released(document, at);
        Ok(())
    }

    /// Focuses the node `target` names, as its `focus()` does; nothing
    /// happens when it has focus already.
    ///
    /// # Errors
    /// As [`Document::ax_find`]; [`AxError::Disabled`];
    /// [`AxError::NotFocusable`].
    pub fn ax_focus(document: &Rc<RefCell<Document>>, target: &AxTarget) -> Result<(), AxError> {
        let found = Self::ax_live(document, target)?;
        if !document.borrow().focusable(found.node) {
            return Err(AxError::NotFocusable {
                role: found.role,
                name: found.name,
            });
        }
        Self::focus(document, Some(found.node));
        Ok(())
    }

    /// Types `text` into the node `target` names, focusing it first:
    /// each character's key goes down, its text arrives, its key comes up.
    ///
    /// # Errors
    /// As [`Document::ax_focus`].
    pub fn ax_type(
        document: &Rc<RefCell<Document>>,
        target: &AxTarget,
        text: &str,
    ) -> Result<(), AxError> {
        Self::ax_focus(document, target)?;
        for c in text.chars() {
            let key = c.to_string();
            Self::key_pressed(document, &key, Modifiers::default());
            Self::character_typed(document, &key);
            Self::key_released(document, &key, Modifiers::default());
        }
        Ok(())
    }

    /// Presses `key` with `modifiers` on the node `target` names,
    /// focusing it first: the key goes down and comes up.
    ///
    /// # Errors
    /// As [`Document::ax_focus`].
    pub fn ax_key(
        document: &Rc<RefCell<Document>>,
        target: &AxTarget,
        key: &str,
        modifiers: Modifiers,
    ) -> Result<(), AxError> {
        Self::ax_focus(document, target)?;
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
}

/// The centre of `bounds`.
fn centre(bounds: BoxRect) -> [f32; 2] {
    [
        bounds.x + bounds.width / 2.0,
        bounds.y + bounds.height / 2.0,
    ]
}
