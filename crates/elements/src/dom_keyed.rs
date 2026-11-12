//! A keyed element's calls: what `create_keyed_element` fixed from the
//! template's `StyleKey` stays as the key says. `set_attribute` of the
//! scope attribute, `class` or `style` is refused naming the site
//! and the fix, and a `class:` directive toggles through
//! `toggle_classes`, which refuses a name the key does not list.

use vapors_dom::NodeId;

use crate::document::Document;

impl Document {
    /// Panics when `name` is one `node`'s key fixes.
    pub(crate) fn refuse_keyed_attribute(&mut self, node: NodeId, name: &str) {
        let Some(key) = self.element_mut(node, "set_attribute").key else {
            return;
        };
        if let Some(refusal) = key.refusal(name) {
            panic!("set_attribute: {refusal}");
        }
    }

    /// The `class` text after `toggles`, or `None` when nothing changes.
    ///
    /// # Panics
    /// An element without a key, or a name its key does not list.
    pub(crate) fn toggled_class(
        &mut self,
        node: NodeId,
        toggles: &[(&str, bool)],
    ) -> Option<String> {
        let element = self.element_mut(node, "toggle_classes");
        let Some(key) = element.key else {
            panic!(
                "toggle_classes: {node:?} was not created by create_keyed_element, so it has no `class:` directives"
            )
        };
        let mask = key
            .toggled(element.toggles, toggles)
            .unwrap_or_else(|refusal| panic!("toggle_classes: {refusal}"));
        if mask == element.toggles {
            return None;
        }
        element.toggles = mask;
        Some(key.class_attribute(mask))
    }
}
