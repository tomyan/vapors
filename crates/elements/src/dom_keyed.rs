//! A keyed element's calls: what `create_keyed_element` fixed from the
//! template's `StyleKey` stays as the key says. `set_attribute` of the
//! scope attribute, `class` or `style` is refused naming the site and
//! the fix, and a `class:` directive toggles through `toggle_classes`,
//! which refuses a name the key does not list. A refused call changes
//! nothing and is kept on the element (`Element::refused`); `check`
//! returns it from every update while the element is in the tree, as
//! it does an unsupported tag. It never panics, so a page built in
//! release reports it as an update error rather than dying.

use vapors_dom::{Dom, NodeId};

use crate::check::describe;
use crate::document::Document;

impl Document {
    /// Keeps `refusal` on `node` and marks it for checking again.
    pub(crate) fn refuse(&mut self, node: NodeId, refusal: String) {
        self.note_change();
        self.element_mut(node, "refuse").refused.push(refusal);
        self.checks.marked.insert(node);
    }

    /// The `class` text after `toggles`, or `None` when nothing changes.
    ///
    /// # Errors
    /// An element without a key, or a name its key does not list.
    pub(crate) fn toggled_class(
        &mut self,
        node: NodeId,
        toggles: &[(&str, bool)],
    ) -> Result<Option<String>, String> {
        let Some(key) = self.style_key(node) else {
            return Err(format!(
                "{} was not created by create_keyed_element, so it has no `class:` directives; create it with create_keyed_element from its template's StyleKey, or set `class` with set_attribute",
                describe(&self.arena, node)
            ));
        };
        let element = self.element_mut(node, "toggle_classes");
        let mask = key.toggled(element.toggles, toggles)?;
        if mask == element.toggles {
            return Ok(None);
        }
        element.toggles = mask;
        Ok(Some(key.class_attribute(mask)))
    }
}
