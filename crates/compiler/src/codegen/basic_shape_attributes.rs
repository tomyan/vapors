//! `<rect>` and `<circle>` attributes refused at build time, naming the
//! file and line (one-scene S4b; #376 decided the same for presentation
//! attributes). The refused cases are an attribute the shape does not
//! take (`transform`, `class`), a literal geometry value the runtime
//! would refuse (a unit, a percentage, a size not above zero), and a
//! required one that is absent. A bound value is checked at update
//! time by the element tree, through the same `vapors_shapes::refusal`.

use vapors_shapes::{allowed, geometry_names, refusal};

use super::attribute_kind::{AttributeKind, kind};
use crate::CompileError;
use crate::ast::{Attribute, Element};
use crate::source::Source;

/// Refuses what `element`'s attributes give it at build time, when it
/// is a basic shape.
pub(super) fn refuse_basic_shape(source: &Source, element: &Element) -> Result<(), CompileError> {
    let tag = element.tag.as_str();
    let (Some(allowed), Some(geometry)) = (allowed(tag), geometry_names(tag)) else {
        return Ok(());
    };
    for attribute in element.attributes.iter() {
        refuse_attribute(source, (tag, allowed, geometry), attribute)?;
    }
    let missing = (geometry.iter())
        .filter(|name| element.attributes.get(name).is_none())
        .find_map(|name| refusal(name, None));
    match missing {
        Some(reason) => Err(source.error(element.offset, format!("<{tag}>: {reason}"))),
        None => Ok(()),
    }
}

/// Refuses `attribute` when the shape does not take it, or when it is a
/// literal geometry value the runtime would refuse.
fn refuse_attribute(
    source: &Source,
    (tag, allowed, geometry): (&str, &[&str], &[&str]),
    attribute: &Attribute,
) -> Result<(), CompileError> {
    let name = attribute.name.as_str();
    let kind = kind(attribute);
    let plain = matches!(
        kind,
        AttributeKind::Literal(_)
            | AttributeKind::Expression(_)
            | AttributeKind::Class
            | AttributeKind::Style
    );
    if plain && !allowed.contains(&name) {
        let message = format!(
            "`{name}` is not an attribute of <{tag}>, which takes {}; remove it",
            allowed.join(", ")
        );
        return Err(source.error(attribute.offset, message));
    }
    let literal = match kind {
        AttributeKind::Literal(value) if geometry.contains(&name) => value,
        _ => return Ok(()),
    };
    match refusal(name, Some(literal)) {
        Some(reason) => Err(source.error(attribute.offset, format!("<{tag}>: {reason}"))),
        None => Ok(()),
    }
}
