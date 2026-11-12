//! `class:name={bool}` on an element: the names go in the element's
//! `StyleKey` as its toggles, and one effect calls `toggle_classes`
//! with each directive's name and expression.

use super::attribute_refusals::refuse_string_styling;
use super::element::attribute_named;
use super::emit::Emitter;
use crate::CompileError;
use crate::ast::{AttrValue, Attribute, Element, Expr};

const PREFIX: &str = "class:";

/// Whether `attribute` is a `class:` directive.
pub(super) fn is_class_directive(attribute: &Attribute) -> bool {
    attribute.name.starts_with(PREFIX)
}

impl Emitter<'_> {
    /// Each `class:` directive's name, in template order, for the key's
    /// toggles; a name that is not a class is refused.
    pub(super) fn directive_names<'e>(
        &self,
        element: &'e Element,
    ) -> Result<Vec<&'e str>, CompileError> {
        element
            .attributes
            .iter()
            .filter(|attribute| is_class_directive(attribute))
            .map(|attribute| {
                directive_name(attribute)
                    .map_err(|message| self.source.error(attribute.offset, message))
            })
            .collect()
    }

    /// The effect that toggles `var`'s directives, when `element` has any.
    pub(super) fn emit_class_directives(
        &mut self,
        element: &Element,
        var: &str,
    ) -> Result<(), CompileError> {
        let directives = self.class_directives(element)?;
        if directives.is_empty() {
            return Ok(());
        }
        let exprs: Vec<&Expr> = directives.iter().map(|(_, expr)| *expr).collect();
        let clones = self.clones_for(&exprs)?;
        let mut pairs = Vec::new();
        for (name, expr) in &directives {
            pairs.push(format!("({name:?}, {})", self.raw(expr)?));
        }
        let body = vec![format!(
            "dom.borrow_mut().toggle_classes({var}, &[{}]);",
            pairs.join(", ")
        )];
        self.emit_effect(&clones, &body);
        Ok(())
    }

    /// The element's literal `class` names for its key; refused when
    /// one is also a directive's class. `class={..}` was refused before
    /// this (`attribute_refusals`, c1b).
    pub(super) fn literal_classes<'e>(
        &self,
        element: &'e Element,
    ) -> Result<Vec<&'e str>, CompileError> {
        let Some(class) = attribute_named(element, "class") else {
            return Ok(Vec::new());
        };
        let AttrValue::Literal(value) = &class.value else {
            // `emit_element` refuses `class={..}` first; a caller that
            // did not still refuses here, naming the site
            refuse_string_styling(self.source, class)?;
            return Err(self.source.error(class.offset, "`class={..}` cannot be part of the element's style key: write class=\"name\" and class:name={condition}"));
        };
        for attribute in element.attributes.iter().filter(|a| is_class_directive(a)) {
            let name = &attribute.name[PREFIX.len()..];
            if value.split_ascii_whitespace().any(|set| set == name) {
                return Err(self.source.error(
                    attribute.offset,
                    format!("`{PREFIX}{name}` toggles a class the literal `class` already sets: remove `{name}` from `class=\"...\"`"),
                ));
            }
        }
        Ok(value.split_ascii_whitespace().collect())
    }

    fn class_directives<'e>(
        &self,
        element: &'e Element,
    ) -> Result<Vec<(&'e str, &'e Expr)>, CompileError> {
        element
            .attributes
            .iter()
            .filter(|attribute| is_class_directive(attribute))
            .map(|attribute| self.class_directive(attribute))
            .collect()
    }

    fn class_directive<'e>(
        &self,
        attribute: &'e Attribute,
    ) -> Result<(&'e str, &'e Expr), CompileError> {
        let name = directive_name(attribute)
            .map_err(|message| self.source.error(attribute.offset, message))?;
        let AttrValue::Expr(expr) = &attribute.value else {
            return Err(self.source.error(
                attribute.offset,
                format!(
                    "`{PREFIX}{name}` takes a bool expression: write {PREFIX}{name}={{condition}}"
                ),
            ));
        };
        Ok((name, expr))
    }
}

/// The directive's class name; an error message when it is not one.
fn directive_name(attribute: &Attribute) -> Result<&str, String> {
    let name = &attribute.name[PREFIX.len()..];
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '-' | '_'))
    {
        return Err(format!(
            "`{PREFIX}{name}` names class `{name}`: a class directive's name is letters, digits, `-` and `_`"
        ));
    }
    Ok(name)
}
