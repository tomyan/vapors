//! The modifier keys held with a key or a pointer event: the DOM's
//! shiftKey, ctrlKey, altKey and metaKey.

/// The modifier keys held with an event.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // the DOM's shiftKey, ctrlKey, altKey and metaKey, one each
pub struct Modifiers {
    /// Shift.
    pub shift: bool,
    /// Control.
    pub control: bool,
    /// Alt (Option on macOS).
    pub alt: bool,
    /// Meta: Command on macOS, the Windows or Super key elsewhere.
    pub meta: bool,
}
