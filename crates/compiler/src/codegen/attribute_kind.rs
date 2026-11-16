//! One answer to what an attribute of an element is, read by the
//! emitter (`element.rs`), the key (`style_key.rs`) and the styles
//! analysis (`styles/facts.rs`): a kind is in the key, set at run
//! time, a listener, a binding or refused, never two of these.

use crate::ast::{AttrValue, Attribute, Attributes, Expr};

/// What one attribute of an element means.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum AttributeKind<'a> {
    /// `bind:this={signal}`: set once the element is whole.
    BindThis,
    /// `class:name={bool}`: a directive the key lists as a toggle.
    ClassDirective(&'a str),
    /// `class="..."`: the key's literal classes.
    Class,
    /// `style="..."`: the key's literal style.
    Style,
    /// The scope attribute, which mount sets: refused where written.
    Scope,
    /// `on<event>={handler}`: a listener for `event`.
    Listener(&'a str),
    /// `bind:<property>={signal}`: a listener and a property effect.
    Binding(&'a str),
    /// `name="value"`: fixed by the key (`StyleKey::attributes`).
    Literal(&'a str),
    /// `name={expr}`: set at run time by an effect.
    Expression(&'a Expr),
}

/// The kind of `attribute`, in the order the emitter tests them.
pub(crate) fn kind(attribute: &Attribute) -> AttributeKind<'_> {
    let name = attribute.name.as_str();
    if name == "bind:this" {
        return AttributeKind::BindThis;
    }
    if let Some(directive) = name.strip_prefix("class:") {
        return AttributeKind::ClassDirective(directive);
    }
    if name.eq_ignore_ascii_case("class") {
        return AttributeKind::Class;
    }
    if name.eq_ignore_ascii_case("style") {
        return AttributeKind::Style;
    }
    if name.eq_ignore_ascii_case(crate::SCOPE_ATTRIBUTE) {
        return AttributeKind::Scope;
    }
    if let Some(event) = name.strip_prefix("on").filter(|rest| !rest.is_empty()) {
        return AttributeKind::Listener(event);
    }
    if let Some(property) = name.strip_prefix("bind:") {
        return AttributeKind::Binding(property);
    }
    match &attribute.value {
        AttrValue::Literal(value) => AttributeKind::Literal(value),
        AttrValue::Expr(expr) => AttributeKind::Expression(expr),
    }
}

/// Whether `name` is `class` or `style`, which a key carries as its
/// own fields (a component tag styles its anchor with them).
pub(crate) fn carried_by_key(name: &str) -> bool {
    name.eq_ignore_ascii_case("class") || name.eq_ignore_ascii_case("style")
}

/// The (name, value) pairs an element's key fixes under
/// `StyleKey::attributes`, in template order.
pub(crate) fn fixed_attributes(attributes: &Attributes) -> Vec<(&str, &str)> {
    attributes
        .iter()
        .filter_map(|attribute| match kind(attribute) {
            AttributeKind::Literal(value) => Some((attribute.name.as_str(), value)),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
#[path = "attribute_kind_tests.rs"]
mod tests;
