//! Pressing one node of the a11y tree (R1) as a person does: the
//! pointer moves to its centre, goes down and comes up there. A node
//! outside its scroll containers' view is scrolled into it first
//! (`a11y_scroll`) and the document updated; a press that still could
//! not reach the node is refused, naming why.

use std::cell::RefCell;
use std::rc::Rc;

use crate::a11y_error::AxError;
use crate::a11y_target::AxTarget;
use crate::a11y_tree::AxElement;
use crate::document::Document;
use crate::layout::BoxRect;

impl Document {
    /// Presses the node `target` names: scrolled into view, the pointer
    /// moves to its centre, goes down and comes up there.
    ///
    /// # Errors
    /// As [`Document::ax_find`]; [`AxError::Disabled`];
    /// [`AxError::NoPointer`]; [`AxError::OutOfView`];
    /// [`AxError::Covered`] when the centre reaches another element;
    /// [`AxError::UpdateFailed`] when the update after a scroll does.
    pub fn ax_press(document: &Rc<RefCell<Document>>, target: &AxTarget) -> Result<(), AxError> {
        let found = Self::ax_reachable(document, target)?;
        let at = centre(found.bounds);
        Self::pointer_moved(document, at);
        Self::pointer_pressed(document, at);
        Self::pointer_released(document, at);
        Ok(())
    }

    /// The node `target` names, scrolled into view, when a press at its
    /// centre reaches it.
    fn ax_reachable(
        document: &Rc<RefCell<Document>>,
        target: &AxTarget,
    ) -> Result<AxElement, AxError> {
        let found = Self::ax_live(document, target)?;
        let scrolled = document
            .borrow_mut()
            .scroll_into_view(found.node, found.bounds);
        if !scrolled {
            return document.borrow().ax_reaches(found);
        }
        document.borrow_mut().update_again(&found)?;
        let found = Self::ax_live(document, target)?;
        document.borrow().ax_reaches(found)
    }

    /// Updates at the last update's viewport, after a scroll for `found`.
    fn update_again(&mut self, found: &AxElement) -> Result<(), AxError> {
        let viewport = self.kept.as_ref().ok_or(AxError::NotUpdated)?.viewport;
        self.update(viewport)
            .map(|_| ())
            .map_err(|errors| AxError::UpdateFailed {
                after: format!("scrolling {} \"{}\" into view", found.role, found.name),
                errors: errors.iter().map(ToString::to_string).collect(),
            })
    }

    /// `found`, when a press at its centre reaches it.
    fn ax_reaches(&self, found: AxElement) -> Result<AxElement, AxError> {
        let at = centre(found.bounds);
        if !self.takes_pointer(found.node) {
            return Err(AxError::NoPointer {
                role: found.role,
                name: found.name,
            });
        }
        if !self.in_viewport(at) {
            return Err(AxError::OutOfView {
                role: found.role,
                name: found.name,
                at,
            });
        }
        self.ax_uncovered(found, at)
    }

    /// `found`, when the element hit at `at` is it or inside it.
    fn ax_uncovered(&self, found: AxElement, at: [f32; 2]) -> Result<AxElement, AxError> {
        let hit = self.hit(at);
        if hit.is_some_and(|hit| self.chain(hit).contains(&found.node)) {
            return Ok(found);
        }
        Err(AxError::Covered {
            role: found.role,
            name: found.name,
            at,
            by: hit.and_then(|hit| self.tag(hit)).map(str::to_owned),
        })
    }

    /// Whether `node` has `pointer-events: auto` in the last update.
    fn takes_pointer(&self, node: vapors_dom::NodeId) -> bool {
        let kept = self.kept.as_ref();
        kept.and_then(|kept| kept.used.get(&node))
            .is_some_and(|used| used.takes_pointer)
    }

    /// Whether `at` is inside the last update's viewport.
    fn in_viewport(&self, at: [f32; 2]) -> bool {
        self.kept.as_ref().is_some_and(|kept| {
            let viewport = kept.viewport;
            (0.0..viewport.width).contains(&at[0]) && (0.0..viewport.height).contains(&at[1])
        })
    }
}

/// The centre of `bounds`.
fn centre(bounds: BoxRect) -> [f32; 2] {
    [
        bounds.x + bounds.width / 2.0,
        bounds.y + bounds.height / 2.0,
    ]
}
