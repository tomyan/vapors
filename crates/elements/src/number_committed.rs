//! What each number field last committed (3-E3c): its number, which a
//! text that is no number falls back to, and its value string, which
//! `change` compares against. HTML fires `change` when the value
//! differs from the value at the last `change` (or at focus): it
//! compares strings, so "3" then "3.0" is a change (HTML 4.10.5.5,
//! "the change event"; Blink's `text_as_of_last_form_control_change_event`).

use vapors_dom::NodeId;

use crate::document::Document;
use crate::number::Field;

/// A number field's last committed number and value string.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Committed {
    pub(crate) number: f64,
    pub(crate) value: String,
}

impl Document {
    /// The number the field last committed, from the field's text when
    /// it has not yet.
    pub(crate) fn last_committed(&self, node: NodeId, field: &Field) -> f64 {
        self.committed
            .get(&node)
            .map(|committed| committed.number)
            .or_else(|| field.committed())
            .unwrap_or_else(|| field.clamped(0.0))
    }

    /// Records `number` and the field's value string now as `node`'s
    /// last committed; the value string recorded before, if any.
    pub(crate) fn remember_committed(&mut self, node: NodeId, number: f64) -> Option<String> {
        let value = self.number_field(node)?.value();
        let last = self.committed.insert(node, Committed { number, value });
        last.map(|committed| committed.value)
    }
}
