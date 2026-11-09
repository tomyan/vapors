//! Setting a value through the a11y tree (R1), as Chromium's
//! accessibility set-value action does, read from Chromium's source
//! (`third_party/blink/renderer`, main, 2026-11-09):
//!
//! - a range (`AXSlider::OnNativeSetValueAction`): nothing when the
//!   text equals the value; else `HTMLInputElement::SetValue` with
//!   `kDispatchInputAndChangeEvent`, which sanitizes it
//!   (`RangeInputType::SanitizeValue`: `StepRange::ClampValue`, clamped
//!   then snapped to a step, as `Range::snapped`) and, when the value
//!   changed, fires `input` then `change` (`InputType::SetValue`). The
//!   action's own `DispatchFormControlChangeEvent` after it fires
//!   nothing more: `TextControlElement::DispatchFormControlChangeEvent`
//!   fires only while a value from before a user edit is held, and the
//!   first `change` cleared it. One `input`, one `change`.
//! - a number field (`AXNodeObject::OnNativeSetValueAction`, a text
//!   field): `SetValue` with the same behaviour; the text is kept as
//!   given, neither clamped nor stepped (`NumberInputType::SanitizeValue`);
//!   when it changed, `input` (`TextFieldInputType::SetValue`), then
//!   `change` when it differs from the value before the user's edits,
//!   as vapors' last committed number.
//!
//! The events oracle cannot play an accessibility action, so the tests
//! pin these rules on vapors' result directly. Where Chromium sanitizes
//! text that is no number (to the range's middle, or an empty field),
//! vapors refuses it, naming it.

use std::cell::RefCell;
use std::rc::Rc;

use crate::a11y_error::AxError;
use crate::a11y_target::AxTarget;
use crate::document::Document;

impl Document {
    /// Sets the value of the slider or spinbutton `target` names to
    /// `value`, as an accessibility set-value does (module docs).
    ///
    /// # Errors
    /// As [`Document::ax_find`]; [`AxError::Disabled`];
    /// [`AxError::NotSettable`] for a node that is no range or number
    /// field; [`AxError::NotANumber`].
    pub fn ax_set_value(
        document: &Rc<RefCell<Document>>,
        target: &AxTarget,
        value: &str,
    ) -> Result<(), AxError> {
        let found = Self::ax_live(document, target)?;
        let node = found.node;
        let (range, number) = {
            let doc = document.borrow();
            (doc.range(node).is_some(), doc.number_field(node).is_some())
        };
        let (role, name) = (found.role, found.name);
        if !(range || number) {
            return Err(AxError::NotSettable { role, name });
        }
        let Some(parsed) = crate::range::parse(value) else {
            let value = value.to_owned();
            return Err(AxError::NotANumber { role, name, value });
        };
        if range {
            crate::range_input::set(document, node, parsed);
        } else {
            crate::number_input::set(document, node, value, parsed);
        }
        Ok(())
    }
}
