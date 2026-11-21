//! The parse-time refusals of `style:<property>={expr}` (decision
//! 2026-11-20-style-bindings): a computed property name, a binding on a
//! component tag, a property bound twice. The refusals that need the
//! property table are made where the key is read (`codegen::style_binding`).

use super::Parser;
use crate::CompileError;
use crate::ast::Attributes;

impl Parser<'_> {
    /// `style:` followed by no name, as in `style:{name}={..}`.
    pub(super) fn computed_style_property(&self, offset: usize) -> CompileError {
        self.error(
            offset,
            format!(
                "`style:{{..}}` computes the property name: write the CSS property itself after the colon, style:height={{expr}}; a property changes only through a template binding with a literal name; {}",
                self.source.check_hint()
            ),
        )
    }

    /// `style:name` given twice on one element.
    pub(super) fn bound_twice(&self, name: &str, offset: usize) -> CompileError {
        self.error(
            offset,
            format!(
                "`{name}` is bound twice on one element: keep one binding of each property; {}",
                self.source.check_hint()
            ),
        )
    }

    /// A component tag takes no `style:` binding: its anchor's styles
    /// are fixed in the template (`2026-11-18-style-variants.md`, 4).
    pub(super) fn refuse_style_bindings_on_component(
        &self,
        tag: &str,
        attributes: &Attributes,
    ) -> Result<(), CompileError> {
        let Some(attribute) = (attributes.iter()).find(|a| a.name.starts_with("style:")) else {
            return Ok(());
        };
        Err(self.error(
            attribute.offset,
            format!(
                "`{}` is on the component tag <{tag}>: a style binding is for an element, so bind it on an element inside the component, or toggle a class with class:name={{condition}} on the tag; {}",
                attribute.name,
                self.source.check_hint()
            ),
        ))
    }
}
