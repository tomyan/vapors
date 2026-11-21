//! `<rect>` and `<circle>` attributes refused at build time, naming the
//! file and line (one-scene S4b; #376 decided the same for presentation
//! attributes). Refused:
//! - an attribute the shape does not take (`transform`, `class`);
//! - a `class:` directive, which would write `class` at run time;
//! - a binding or a listener: a shape has no value to bind and no box to
//!   hit (it is drawn as part of its svg), so a listener would never run;
//! - a literal geometry value the runtime would refuse (a unit, a
//!   percentage, a negative size or radius);
//! - a required size that is absent.
//!
//! `bind:this` is taken: it hands the node to the script and sets
//! nothing. A bound geometry value is checked at update time by the
//! element tree, through the same `vapors_shapes::refusal`. A size of 0
//! draws nothing, as in SVG, and is not refused.

use vapors_shapes::{allowed, geometry_names, not_taken, refusal};

use super::attribute_kind::{AttributeKind, kind};
use crate::CompileError;
use crate::ast::{Attribute, Element};
use crate::source::Source;

/// Refuses what `element`'s attributes give it at build time, when it
/// is a basic shape.
pub(super) fn refuse_basic_shape(source: &Source, element: &Element) -> Result<(), CompileError> {
    let tag = element.tag.as_str();
    let Some(geometry) = geometry_names(tag) else {
        return Ok(());
    };
    for attribute in element.attributes.iter() {
        refuse_kind(source, tag, attribute)?;
        refuse_literal(source, (tag, geometry), attribute)?;
    }
    let missing = (geometry.iter())
        .filter(|name| element.attributes.get(name).is_none())
        .find_map(|name| refusal(name, None));
    match missing {
        Some(reason) => Err(source.error(element.offset, format!("<{tag}>: {reason}"))),
        None => Ok(()),
    }
}

/// Refuses `attribute` when the shape does not take it: a plain
/// attribute outside [`allowed`], a `class:` directive, a binding or a
/// listener.
fn refuse_kind(source: &Source, tag: &str, attribute: &Attribute) -> Result<(), CompileError> {
    let name = attribute.name.as_str();
    let takes = allowed(tag).expect("a basic shape").contains(&name);
    let refused = match kind(attribute) {
        AttributeKind::Literal(_)
        | AttributeKind::Expression(_)
        | AttributeKind::Class
        | AttributeKind::Style => !takes,
        AttributeKind::ClassDirective(_)
        | AttributeKind::StyleBinding(_)
        | AttributeKind::Binding(_)
        | AttributeKind::Listener(_) => true,
        AttributeKind::BindThis | AttributeKind::Scope => false,
    };
    if refused {
        return Err(source.error(attribute.offset, not_taken(tag, name)));
    }
    Ok(())
}

/// Refuses `attribute` when it is a literal geometry value the runtime
/// would refuse.
fn refuse_literal(
    source: &Source,
    (tag, geometry): (&str, &[&str]),
    attribute: &Attribute,
) -> Result<(), CompileError> {
    let literal = match kind(attribute) {
        AttributeKind::Literal(value) if geometry.contains(&attribute.name.as_str()) => value,
        _ => return Ok(()),
    };
    match refusal(&attribute.name, Some(literal)) {
        Some(reason) => Err(source.error(attribute.offset, format!("<{tag}>: {reason}"))),
        None => Ok(()),
    }
}
