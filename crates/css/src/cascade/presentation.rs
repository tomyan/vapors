//! SVG presentation attributes (SVG 2 section 6.6, Cascade Level 4
//! section 6.4.3): `fill`, `stroke` and `stroke-width` written as
//! attributes on an `<svg>` or `<path>` are author declarations ranked
//! below every author rule, specificity 0 and first in order, as
//! Chromium ranks them. One answer for the runtime cascade and the
//! build-time analysis, which reads a key's literal attributes through
//! [`declarations_of`].

use crate::ast::Declaration;
use crate::selector::{Element, Specificity};
use crate::{Error, Result};

/// The sheet index `ComputedStyle::declared_in` gives a declaration
/// from a presentation attribute.
pub const PRESENTATION_SHEET: usize = usize::MAX - 1;

/// Below any selector's specificity but `*`'s, which ties and wins by
/// coming later.
pub(crate) const PRESENTATION_SPECIFICITY: Specificity = Specificity {
    ids: 0,
    classes: 0,
    types: 0,
};

/// The presentation attributes vapors maps; each is the property of
/// its name.
pub const PRESENTATION_ATTRIBUTES: [&str; 3] = ["fill", "stroke", "stroke-width"];

/// The elements that take them: the SVG elements vapors draws.
const SVG_TAGS: [&str; 2] = ["svg", "path"];

/// The element's presentation attributes as declarations; empty for an
/// element that is not an SVG element vapors draws.
pub(crate) fn declarations<E: Element>(element: &E) -> Result<Vec<Declaration>> {
    declarations_of(element.tag_name(), |name| element.attribute(name))
}

/// The presentation attributes `attribute` gives an element of `tag`,
/// as declarations, in [`PRESENTATION_ATTRIBUTES`] order.
///
/// # Errors
/// An attribute whose value is not one value of its property, naming
/// the attribute, the element and the value.
pub fn declarations_of<'a>(
    tag: &str,
    attribute: impl Fn(&str) -> Option<&'a str>,
) -> Result<Vec<Declaration>> {
    if !SVG_TAGS.contains(&tag) {
        return Ok(Vec::new());
    }
    PRESENTATION_ATTRIBUTES
        .iter()
        .filter_map(|&name| attribute(name).map(|value| declaration(tag, name, value)))
        .collect()
}

/// `name="value"` on `<tag>` as one declaration of `name`.
fn declaration(tag: &str, name: &str, value: &str) -> Result<Declaration> {
    let refused = |why: String| {
        Error::new(
            crate::Position::default(),
            format!("the {name} attribute of <{tag}> is {value:?}: {why}"),
        )
    };
    let parsed =
        crate::parse_inline(&format!("{name}: {value}")).map_err(|e| refused(e.message))?;
    match parsed.as_slice() {
        [one] if one.name == name && !one.important => Ok(one.clone()),
        _ => Err(refused(format!(
            "an attribute holds one {name} value, with no `;` or `!important`"
        ))),
    }
}
