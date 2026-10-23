//! The `style` attribute's declarations (Cascade Level 4 section 6.2):
//! author declarations attached to the element, which beat any
//! selector's of the same importance.

use crate::ast::Declaration;
use crate::selector::{Element, Specificity};
use crate::{Error, Result};

/// The sheet index `ComputedStyle::declared_in` gives a declaration
/// from the element's `style` attribute.
pub const INLINE_SHEET: usize = usize::MAX;

/// Above any selector's specificity, so an inline declaration wins
/// within its origin and importance.
pub(crate) const INLINE_SPECIFICITY: Specificity = Specificity {
    ids: u32::MAX,
    classes: u32::MAX,
    types: u32::MAX,
};

/// The element's `style` attribute, parsed; empty without one.
pub(crate) fn declarations<E: Element>(element: &E) -> Result<Vec<Declaration>> {
    let Some(text) = element.attribute("style") else {
        return Ok(Vec::new());
    };
    crate::parse_inline(text).map_err(|error| {
        Error::new(
            error.position,
            format!(
                "in the style attribute of <{}>: {}",
                element.tag_name(),
                error.message
            ),
        )
    })
}
