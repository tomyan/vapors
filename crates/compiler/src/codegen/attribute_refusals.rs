//! Element attributes that would compile to something other than what
//! they say: a boolean attribute from an expression, which is on even
//! when the expression is false, and a colon-named attribute that is
//! not a directive, which would become an attribute with a colon in
//! its name.

use crate::CompileError;
use crate::ast::{AttrValue, Attribute};
use crate::source::Source;

/// The HTML boolean attributes: on by being present, whatever the value.
const BOOLEAN_ATTRIBUTES: &[&str] = &[
    "allowfullscreen",
    "async",
    "autofocus",
    "autoplay",
    "checked",
    "controls",
    "default",
    "defer",
    "disabled",
    "formnovalidate",
    "hidden",
    "inert",
    "ismap",
    "itemscope",
    "loop",
    "multiple",
    "muted",
    "nomodule",
    "novalidate",
    "open",
    "playsinline",
    "readonly",
    "required",
    "reversed",
    "selected",
];

/// The directive prefixes an element takes.
const DIRECTIVES: &[&str] = &["bind:"];

/// Refuses `attribute` on an element when it would not mean what it says.
pub(super) fn refuse_misleading(
    source: &Source,
    attribute: &Attribute,
) -> Result<(), CompileError> {
    refuse_unknown_directive(source, attribute)?;
    refuse_boolean_expression(source, attribute)
}

fn refuse_unknown_directive(source: &Source, attribute: &Attribute) -> Result<(), CompileError> {
    let name = &attribute.name;
    if !name.contains(':') || DIRECTIVES.iter().any(|prefix| name.starts_with(prefix)) {
        return Ok(());
    }
    Err(source.error(
        attribute.offset,
        format!(
            "`{name}` is not a directive: a name with `:` is a directive, and an element takes `bind:value`, `bind:checked` and `bind:this`; a listener is `onclick={{handler}}`, with no colon"
        ),
    ))
}

fn refuse_boolean_expression(source: &Source, attribute: &Attribute) -> Result<(), CompileError> {
    let name = &attribute.name;
    let lowercase = name.to_ascii_lowercase();
    if !matches!(attribute.value, AttrValue::Expr(_))
        || !BOOLEAN_ATTRIBUTES.contains(&lowercase.as_str())
    {
        return Ok(());
    }
    let binding = if lowercase == "checked" {
        "`bind:checked={signal}`, or "
    } else {
        ""
    };
    Err(source.error(
        attribute.offset,
        format!(
            "`{name}={{..}}` would always be on: an HTML boolean attribute is on by being present, so `{name}=\"false\"` is on too, and generated code cannot remove an attribute; write {binding}`{name}=\"\"` on an element inside `{{if ..}}`"
        ),
    ))
}
