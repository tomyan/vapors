//! Element attributes that would compile to something other than what
//! they say: a boolean attribute from an expression, which is on even
//! when the expression is false, and a colon-named attribute that is
//! not a directive, which would become an attribute with a colon in
//! its name; and `class={..}` or `style={..}`, a string the style
//! analysis cannot see into.

use crate::CompileError;
use crate::ast::{AttrValue, Attribute};
use crate::source::Source;

/// The HTML boolean attributes, including those of elements vapors does
/// not implement: on by being present, whatever the value.
const BOOLEAN_ATTRIBUTES: &[&str] = &[
    "allowfullscreen",
    "alpha",
    "async",
    "autofocus",
    "autoplay",
    "checked",
    "compact",
    "controls",
    "declare",
    "default",
    "defer",
    "disablepictureinpicture",
    "disableremoteplayback",
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
    "noresize",
    "noshade",
    "novalidate",
    "nowrap",
    "open",
    "playsinline",
    "readonly",
    "required",
    "reversed",
    "selected",
    "shadowrootclonable",
    "shadowrootdelegatesfocus",
    "shadowrootserializable",
    "truespeed",
    "typemustmatch",
];

/// The directive prefixes an element takes.
const DIRECTIVES: &[&str] = &["bind:", "class:"];

/// Refuses `attribute` on an element when it would not mean what it says.
pub(super) fn refuse_misleading(
    source: &Source,
    attribute: &Attribute,
) -> Result<(), CompileError> {
    refuse_unknown_directive(source, attribute)?;
    refuse_string_styling(source, attribute)?;
    refuse_boolean_literal_false(source, attribute)?;
    refuse_boolean_expression(source, attribute)
}

/// `class={..}` and `style={..}` hide from the style analysis which
/// classes and declarations an element can have.
fn refuse_string_styling(source: &Source, attribute: &Attribute) -> Result<(), CompileError> {
    if !matches!(attribute.value, AttrValue::Expr(_)) {
        return Ok(());
    }
    let message = match attribute.name.to_ascii_lowercase().as_str() {
        "class" => {
            "`class={..}` is refused: the compiler must see every class an element can have; write class=\"name\" for fixed classes and class:name={condition} for each class that toggles"
        }
        "style" => {
            "`style={..}` is refused: the compiler must see every declaration an element can have; write the declarations under a class in the <style> section and toggle it with class:name={condition}"
        }
        _ => return Ok(()),
    };
    Err(source.error(attribute.offset, message))
}

const NAMESPACES: &[&str] = &["xlink:", "xml:", "xmlns:"];

fn refuse_unknown_directive(source: &Source, attribute: &Attribute) -> Result<(), CompileError> {
    let name = &attribute.name;
    if !name.contains(':') || DIRECTIVES.iter().any(|prefix| name.starts_with(prefix)) {
        return Ok(());
    }
    if NAMESPACES.iter().any(|prefix| name.starts_with(prefix)) {
        return Err(source.error(
            attribute.offset,
            format!(
                "`{name}` is a namespaced attribute: namespaced attributes such as `xlink:href`, `xml:lang` and `xmlns:*` are not supported; use the plain form (`href`) where the element accepts it"
            ),
        ));
    }
    Err(source.error(
        attribute.offset,
        format!(
            "`{name}` is not a directive: a name with `:` is a directive, and an element takes `bind:value`, `bind:checked`, `bind:this` and `class:name={{bool}}`; a listener is `onclick={{handler}}`, with no colon"
        ),
    ))
}

fn refuse_boolean_literal_false(
    source: &Source,
    attribute: &Attribute,
) -> Result<(), CompileError> {
    let name = &attribute.name;
    let AttrValue::Literal(value) = &attribute.value else {
        return Ok(());
    };
    if !is_boolean_attribute(name) || !value.eq_ignore_ascii_case("false") {
        return Ok(());
    }
    Err(source.error(
        attribute.offset,
        format!(
            "`{name}=\"{value}\"` is on: an HTML boolean attribute is on by being present, whatever its value; omit it, or write `{name}=\"\"` on an element inside `{{if ..}}`"
        ),
    ))
}

fn is_boolean_attribute(name: &str) -> bool {
    BOOLEAN_ATTRIBUTES.contains(&name.to_ascii_lowercase().as_str())
}

fn refuse_boolean_expression(source: &Source, attribute: &Attribute) -> Result<(), CompileError> {
    let name = &attribute.name;
    let lowercase = name.to_ascii_lowercase();
    if !matches!(attribute.value, AttrValue::Expr(_)) || !is_boolean_attribute(name) {
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
