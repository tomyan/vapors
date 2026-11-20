//! `Dom::bind_style`: a binding writes the element's own declaration,
//! read by the cascade as inline style (the key's literal text, then
//! the bound declarations in property order). It restyles that element
//! alone and marks its box; a colour then asks layout for nothing. The
//! same value changes nothing. A keyed element's `style` attribute has
//! two writers only, the key and this (`set_attribute` refuses it), so
//! `Element::bound` and the attribute cannot diverge.

use crate::document::Document;
use vapors_dom::{BoundProperty, BoundValue, NodeId};

impl Document {
    /// Binds `property` of `node` to `value`.
    ///
    /// # Errors
    /// A node without a key, a property its key lacks, or a bad value.
    pub(crate) fn bound_style(
        &mut self,
        node: NodeId,
        property: BoundProperty,
        value: BoundValue,
    ) -> Result<(), String> {
        let element = self.element_mut(node, "bind_style");
        let Some(key) = element.key else {
            return Err(vapors_dom::unkeyed_binding_refusal(&element.tag));
        };
        let declared = key
            .bound_declaration(property, value)
            .map_err(|why| format!("{why} (element <{}>)", key.tag))?;
        match element.bound.iter_mut().find(|(set, _)| *set == property) {
            Some((_, text)) if *text == declared => return Ok(()),
            Some((_, text)) => *text = declared,
            None => element.bound.push((property, declared)),
        }
        element.bound.sort_by_key(|(set, _)| *set);
        let bound = element.bound.iter().map(|(_, text)| text.as_str());
        let style: Vec<&str> = key.style.into_iter().chain(bound).collect();
        let style = style.join("; ");
        element.attributes.insert("style".to_owned(), style);
        self.note_change();
        self.dirty.element(node);
        self.boxes.marks.mark(node);
        Ok(())
    }
}
