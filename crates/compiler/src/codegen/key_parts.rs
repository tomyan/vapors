//! The parts of a `StyleKey` read from a template element's
//! attributes, with every refusal codegen makes for them: the one
//! reading the emitted key (`style_key.rs`) and the styles analysis
//! (`styles/facts.rs`) share, so what the analysis judges fixed is
//! what the key fixes.

use super::attribute_kind::fixed_attributes;
use super::attribute_refusals::refuse_string_styling;
use super::element::attribute_named;
use super::element_class::{directive_names, literal_classes};
use super::style_key::Keyed;
use crate::CompileError;
use crate::ast::{AttrValue, Attribute};
use crate::source::Source;

/// What the key fixes, borrowed from the attributes.
#[derive(Debug)]
pub(crate) struct KeyParts<'e> {
    /// The literal `class` names.
    pub(crate) classes: Vec<&'e str>,
    /// The `class:` directive names.
    pub(crate) toggles: Vec<&'e str>,
    /// The literal `style` text.
    pub(crate) style: Option<&'e str>,
    /// Every other literal attribute, in template order; none for a
    /// component tag, whose other attributes are props.
    pub(crate) attributes: Vec<(&'e str, &'e str)>,
}

/// `keyed`'s parts, refusing what a key cannot hold: `class={..}`,
/// `style={..}`, a directive that is also a literal class, a directive
/// name that is not a class, more than `MAX_CLASS_DIRECTIVES`.
pub(crate) fn key_parts<'e>(
    source: &Source,
    keyed: &Keyed<'e>,
) -> Result<KeyParts<'e>, CompileError> {
    Ok(KeyParts {
        classes: literal_classes(source, keyed.attributes)?,
        toggles: key_directives(source, keyed)?,
        style: key_style(source, keyed)?,
        attributes: if keyed.carries_attributes {
            fixed_attributes(keyed.attributes)
        } else {
            Vec::new()
        },
    })
}

/// The literal `style`; `style={..}` is refused here, where it would
/// otherwise be dropped.
fn key_style<'e>(source: &Source, keyed: &Keyed<'e>) -> Result<Option<&'e str>, CompileError> {
    match attribute_named(keyed.attributes, "style") {
        Some(
            attribute @ Attribute {
                value: AttrValue::Expr(_),
                ..
            },
        ) => {
            refuse_string_styling(source, attribute)?;
            unreachable!("refuse_string_styling refuses every `style={{..}}`")
        }
        Some(Attribute {
            value: AttrValue::Literal(text),
            ..
        }) => Ok(Some(text)),
        None => Ok(None),
    }
}

/// `keyed`'s directive names, refusing more than
/// [`crate::MAX_CLASS_DIRECTIVES`], naming the element.
fn key_directives<'e>(source: &Source, keyed: &Keyed<'e>) -> Result<Vec<&'e str>, CompileError> {
    let names = directive_names(source, keyed.attributes)?;
    if names.len() > crate::MAX_CLASS_DIRECTIVES {
        return Err(source.error(
            keyed.offset,
            format!(
                "<{}> has {} `class:` directives; at most {} fit one element, so move some to {}",
                keyed.name,
                names.len(),
                crate::MAX_CLASS_DIRECTIVES,
                keyed.elsewhere
            ),
        ));
    }
    Ok(names)
}
