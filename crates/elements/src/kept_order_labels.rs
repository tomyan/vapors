//! The kept order's labels (`kept_order.rs`, `paint_labels.rs`): a step's
//! position along the steps found from its label, which the splice's
//! edits are made at.

use super::KeptOrder;
use crate::paint_labels::Label;
use crate::paint_order::Step;

/// A kept step's label in paint order, if it has one (`DrawnSteps::label_of`).
pub(crate) type LabelOf<'a> = &'a dyn Fn(Step) -> Option<Label>;

impl KeptOrder {
    /// The position along `steps` of the step labelled `label`, or a
    /// panic naming it: the drawing steps label only steps of this order.
    pub(crate) fn position_of(&self, label: Label) -> usize {
        self.labels.binary_search(&label).unwrap_or_else(|_| {
            panic!(
                "label {label} names no step of the kept paint order: the drawing steps were \
                 labelled from another order (paint_partial.rs); reproduce with `cargo xtask test`"
            )
        })
    }
}
