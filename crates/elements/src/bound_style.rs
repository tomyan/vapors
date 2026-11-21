//! `Dom::bind_style`: a binding writes the element's own declaration,
//! read by the cascade as inline style (the key's literal text, then
//! the bound declarations in property order). It restyles that element
//! alone and marks its box; a colour then asks layout for nothing. The
//! same value changes nothing. A keyed element's `style` attribute has
//! two writers only, the key and this (`set_attribute` refuses it), so
//! `Element::bound` and the attribute cannot diverge.

use crate::check::describe;
use crate::document::Document;
use vapors_dom::{BoundProperty, BoundValue, NodeId};

impl Document {
    /// The `style` text after binding `property` to `value`, or `None`
    /// when the value is the one already bound.
    ///
    /// # Errors
    /// A node without a key, a property its key lacks, or a bad value;
    /// nothing is kept.
    pub(crate) fn bound_style(
        &mut self,
        node: NodeId,
        property: BoundProperty,
        value: BoundValue,
    ) -> Result<Option<String>, String> {
        self.element_mut(node, "bind_style");
        let what = describe(&self.arena, node);
        let element = self.element_mut(node, "bind_style");
        let Some(key) = element.key else {
            return Err(vapors_dom::unkeyed_binding_refusal(&what));
        };
        let declared = key.bound_declaration(property, value)?;
        match element.bound.iter_mut().find(|(set, _)| *set == property) {
            Some((_, text)) if *text == declared => return Ok(None),
            Some((_, text)) => *text = declared,
            None => element.bound.push((property, declared)),
        }
        element.bound.sort_by_key(|(set, _)| *set);
        let bound = element.bound.iter().map(|(_, text)| text.as_str());
        let style: Vec<&str> = key.style.into_iter().chain(bound).collect();
        Ok(Some(style.join("; ")))
    }
}
