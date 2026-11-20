//! `Dom::bind_style`: a template's `style:<property>={expr}` writes the
//! element's own declaration, which the cascade reads as inline style
//! (`attribute("style")`: the key's literal text, then the bound
//! declarations), so it beats every selector. A change restyles that
//! element alone, and its box is marked for layout; whether layout
//! does work follows from the computed style differing, so a colour
//! asks none. A value equal to the bound one changes nothing.

use vapors_dom::{BoundProperty, BoundValue, NodeId};

use crate::document::Document;

impl Document {
    /// Binds `property` of `node` to `value`.
    ///
    /// # Errors
    /// A node without a key, a property its key does not list, or a
    /// value outside the property's domain: the refusal, to keep.
    pub(crate) fn bound_style(
        &mut self,
        node: NodeId,
        property: BoundProperty,
        value: BoundValue,
    ) -> Result<(), String> {
        let element = self.element_mut(node, "bind_style");
        let Some(key) = element.key else {
            return Err(format!(
                "<{}> was not created by create_keyed_element, so it has no `style:` binding; create it from its template's StyleKey",
                element.tag
            ));
        };
        let declared = key
            .bound_declaration(property, value)
            .map_err(|why| format!("{why} (element <{}>)", key.tag))?;
        match element.bound.iter_mut().find(|(set, _)| *set == property) {
            Some((_, text)) if *text == declared => return Ok(()),
            Some((_, text)) => *text = declared,
            None => element.bound.push((property, declared)),
        }
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
