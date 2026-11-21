//! `style:<property>={expr}` on an element (decision
//! 2026-11-20-style-bindings): the properties go in the element's
//! `StyleKey` as `bound`, and each binding is one effect that calls
//! `Dom::bind_style` with the typed value, so a change reruns only the
//! binding whose expression changed. Every refusal is made here, where
//! the key is read (`key_parts`), so the styles analysis and the
//! emitted key agree: a property that is not bindable, a quoted value,
//! and a literal of a type the property cannot take.

use vapors_dom::{BoundKind, BoundProperty};

use super::attribute_kind::{AttributeKind, kind};
use super::emit::Emitter;
use super::syn_map::parse_expr;
use crate::CompileError;
use crate::ast::{AttrValue, Attribute, Attributes, Expr};
use crate::source::Source;

const DOM: &str = "vapors_elements::__wisp::dom";

/// What a kind takes, as messages say it.
fn takes(kind: BoundKind) -> &'static str {
    match kind {
        BoundKind::Length => "a px f32",
        BoundKind::Color => "an Rgba",
    }
}

/// The element's bindings in template order, each with its attribute.
pub(crate) fn bindings<'e>(
    source: &Source,
    attributes: &'e Attributes,
) -> Result<Vec<(BoundProperty, &'e Attribute)>, CompileError> {
    let named = attributes
        .iter()
        .filter_map(|attribute| match kind(attribute) {
            AttributeKind::StyleBinding(name) => Some((name, attribute)),
            _ => None,
        });
    named
        .map(|(name, attribute)| {
            let property = bindable(source, name, attribute)?;
            check_value(source, property, attribute)?;
            Ok((property, attribute))
        })
        .collect()
}

fn bindable(
    source: &Source,
    name: &str,
    attribute: &Attribute,
) -> Result<BoundProperty, CompileError> {
    BoundProperty::from_css(name).ok_or_else(|| {
        let all: Vec<&str> = BoundProperty::ALL.iter().map(|p| p.css_name()).collect();
        source.error(
            attribute.offset,
            format!(
                "`style:{name}` is not a bindable property: bind one of {}; for any other property write it under a class in the <style> section and toggle it with class:name={{condition}}; {}",
                all.join(", "),
                source.check_hint()
            ),
        )
    })
}

/// Refuses a quoted value and a literal the property cannot take.
fn check_value(
    source: &Source,
    property: BoundProperty,
    attribute: &Attribute,
) -> Result<(), CompileError> {
    let (name, wants) = (property.css_name(), takes(property.kind()));
    let AttrValue::Expr(expr) = &attribute.value else {
        return Err(source.error(
            attribute.offset,
            format!(
                "`style:{name}` takes an expression of {wants}: write style:{name}={{expr}}, not a quoted value; {}",
                source.check_hint()
            ),
        ));
    };
    let Some(got) = wrong_literal(property.kind(), &parse_expr(source, expr)?) else {
        return Ok(());
    };
    Err(source.error(
        attribute.offset,
        format!(
            "`style:{name}` takes {wants}, not {got}: bind an expression of that type, such as a signal read or a computed value; {}",
            source.check_hint()
        ),
    ))
}

/// What a literal is, when it is not what `kind` takes: a float for a
/// length, nothing for a colour.
fn wrong_literal(kind: BoundKind, expr: &syn::Expr) -> Option<&'static str> {
    match expr {
        syn::Expr::Paren(inner) => wrong_literal(kind, &inner.expr),
        syn::Expr::Group(inner) => wrong_literal(kind, &inner.expr),
        syn::Expr::Unary(inner) => wrong_literal(kind, &inner.expr),
        syn::Expr::Lit(literal) => match (&literal.lit, kind) {
            (syn::Lit::Float(_), BoundKind::Length) => None,
            (syn::Lit::Float(_), BoundKind::Color) => Some("a float literal"),
            (syn::Lit::Int(_), _) => Some("an integer literal"),
            (syn::Lit::Bool(_), _) => Some("a bool literal"),
            (syn::Lit::Str(_), _) => Some("a string literal"),
            _ => Some("a literal of another type"),
        },
        _ => None,
    }
}

impl Emitter<'_> {
    /// One effect per `style:` binding of `attributes`, on `var`.
    pub(super) fn emit_style_bindings(
        &mut self,
        attributes: &Attributes,
        var: &str,
    ) -> Result<(), CompileError> {
        for (property, attribute) in bindings(self.source, attributes)? {
            let AttrValue::Expr(expr) = &attribute.value else {
                unreachable!("`bindings` refuses a quoted value")
            };
            self.emit_style_binding(var, property, expr)?;
        }
        Ok(())
    }

    fn emit_style_binding(
        &mut self,
        var: &str,
        property: BoundProperty,
        expr: &Expr,
    ) -> Result<(), CompileError> {
        let variant = match property.kind() {
            BoundKind::Length => "Length",
            BoundKind::Color => "Color",
        };
        let clones = self.clones_for(&[expr])?;
        let body = vec![
            format!(
                "let value = {DOM}::BoundValue::{variant}({});",
                self.raw(expr)?
            ),
            format!(
                "dom.borrow_mut().bind_style({var}, {DOM}::BoundProperty::{property:?}, value);"
            ),
        ];
        self.emit_effect(&clones, &body);
        Ok(())
    }
}

/// The key's `bound` field: the properties in template order.
pub(crate) fn bound_literal(properties: &[BoundProperty]) -> String {
    let each: Vec<String> = properties
        .iter()
        .map(|property| format!("{DOM}::BoundProperty::{property:?}"))
        .collect();
    each.join(", ")
}
