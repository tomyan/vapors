//! A number field's range validity, as the DOM's
//! `validity.rangeUnderflow` and `validity.rangeOverflow`: a typed
//! number outside `min` and `max` is kept on commit, as Chromium keeps
//! it, so a host reads here whether it is outside, and, when it refuses
//! the number, puts its own back as the last committed.

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

/// Why [`Document::set_committed_number`] refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetNumberError {
    /// The node is not an `input type=number`.
    NotANumberField,
    /// The number is NaN or infinite, which no field can show.
    NotFinite,
}

impl Document {
    /// Show `value` in number field `node` and record it as the field's
    /// last committed number, firing nothing: for a host that refused a
    /// committed number and puts its own back, so the same number typed
    /// again fires `change` again.
    ///
    /// # Errors
    ///
    /// [`SetNumberError`]: `node` is no number field, or `value` is not
    /// finite; nothing changes.
    pub fn set_committed_number(&mut self, node: NodeId, value: f64) -> Result<(), SetNumberError> {
        if self.number_field(node).is_none() {
            return Err(SetNumberError::NotANumberField);
        }
        if !value.is_finite() {
            return Err(SetNumberError::NotFinite);
        }
        self.set_number_text(node, crate::range::written(value));
        self.remember_committed(node, value);
        Ok(())
    }
}
