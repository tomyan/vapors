//! A number field's range validity, as the DOM's
//! `validity.rangeUnderflow` and `validity.rangeOverflow`: a typed
//! number outside `min` and `max` is kept on commit, as Chromium keeps
//! it, so a host reads here whether it is outside.

use vapors_dom::NodeId;

use crate::document::Document;

/// Whether a number field's number is below `min` or above `max`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RangeValidity {
    /// The number is below `min` (`rangeUnderflow`).
    pub underflow: bool,
    /// The number is above `max` (`rangeOverflow`).
    pub overflow: bool,
}

impl Document {
    /// `node`'s range validity, when it is an `input type=number`;
    /// text that is no number is neither under nor over.
    #[must_use]
    pub fn range_validity(&self, node: NodeId) -> Option<RangeValidity> {
        let field = self.number_field(node)?;
        Some(RangeValidity {
            underflow: field.range_underflow(),
            overflow: field.range_overflow(),
        })
    }
}
