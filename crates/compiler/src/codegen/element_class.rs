//! `class:name={bool}` on an element: one effect sets the `class`
//! attribute to `vapors_dom::class_names` of the literal `class` names
//! and each directive's name and expression.

use super::emit::Emitter;
use crate::CompileError;
use crate::ast::{AttrValue, Attribute, Element, Expr};

const PREFIX: &str = "class:";

/// Whether `attribute` is a `class:` directive.
pub(super) fn is_class_directive(attribute: &Attribute) -> bool {
    attribute.name.starts_with(PREFIX)
}

/// Whether `element` has a `class:` directive, so its literal `class`
/// is set by the directives' effect.
pub(super) fn has_class_directives(element: &Element) -> bool {
    element.attributes.iter().any(is_class_directive)
}

impl Emitter<'_> {
    /// The effect that sets `var`'s `class`, when `element` has directives.
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
        let literal = self.literal_class(element)?;
        let body = class_effect_body(&literal, &pairs.join(", "), var);
        self.emit_effect(&clones, &body);
        Ok(())
    }

    /// The element's literal `class` text, or none; refused when it is
    /// an expression the directives' effect would drop, or already sets
    /// a directive's class.
    fn literal_class(&self, element: &Element) -> Result<String, CompileError> {
        let Some(class) = class_attribute(element) else {
            return Ok(String::new());
        };
        let AttrValue::Literal(value) = &class.value else {
            return Err(self.source.error(
                class.offset,
                "`class={..}` beside `class:` directives would be dropped: the directives' effect sets `class` from the literal `class=\"..\"` only; write the names in `class=\"..\"` and toggle with `class:name={bool}`",
            ));
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
        Ok(value.clone())
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
        let name = &attribute.name[PREFIX.len()..];
        let AttrValue::Expr(expr) = &attribute.value else {
            return Err(self.source.error(
                attribute.offset,
                format!(
                    "`{PREFIX}{name}` takes a bool expression: write {PREFIX}{name}={{condition}}"
                ),
            ));
        };
        if name.is_empty()
            || !name
                .chars()
                .all(|c| c.is_alphanumeric() || matches!(c, '-' | '_'))
        {
            return Err(self.source.error(
                attribute.offset,
                format!("`{PREFIX}{name}` names class `{name}`: a class directive's name is letters, digits, `-` and `_`"),
            ));
        }
        Ok((name, expr))
    }
}

/// The element's `class` attribute, in any case.
pub(super) fn class_attribute(element: &Element) -> Option<&Attribute> {
    (element.attributes.iter()).find(|attribute| attribute.name.eq_ignore_ascii_case("class"))
}

/// `vapors_dom::class_names` of the literal names and each directive's
/// `(name, on)`; `on` must be a `bool`.
fn class_effect_body(literal: &str, pairs: &str, var: &str) -> Vec<String> {
    vec![
        format!("let value = vapors_dom::class_names({literal:?}, &[{pairs}]);"),
        format!("dom.borrow_mut().set_attribute_if_changed({var}, \"class\", &value);"),
    ]
}
