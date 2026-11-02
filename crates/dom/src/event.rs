//! The event a listener receives.

use crate::{Capture, Modifiers, NodeId, Pointer};

/// An event delivered to a listener. The payload fields are set by
/// the kind of event: `input` and `change` carry `value`, and on a
/// range or number input the value as a number too; `change` on a
/// checkbox carries `checked`.
#[derive(Debug, Clone, Default)]
pub struct Event {
    name: String,
    value: Option<String>,
    value_number: Option<f64>,
    checked: Option<bool>,
    propagation_stopped: bool,
    default_prevented: bool,
    pointer: Option<Pointer>,
    capture: Option<Capture>,
    target: Option<NodeId>,
    key: Option<String>,
    modifiers: Modifiers,
}

impl Event {
    /// An event of the given kind with no payload.
    #[must_use]
    pub fn new(name: &str) -> Event {
        Event {
            name: name.to_string(),
            ..Event::default()
        }
    }

    /// The modifier keys held with a pointer event: `pointerdown`,
    /// `pointerup`, `click` and `dblclick` carry them.
    #[must_use]
    pub fn with_modifiers(mut self, modifiers: Modifiers) -> Event {
        self.modifiers = modifiers;
        self
    }

    /// The modifier keys held: the DOM's altKey, ctrlKey, metaKey and
    /// shiftKey.
    #[must_use]
    pub fn modifiers(&self) -> Modifiers {
        self.modifiers
    }

    /// Attaches the `value` payload.
    #[must_use]
    pub fn with_value(mut self, value: &str) -> Event {
        self.value = Some(value.to_string());
        self
    }

    /// Attaches the value as a number, on an `input` or `change` from a
    /// range or number input.
    #[must_use]
    pub fn with_value_number(mut self, value: f64) -> Event {
        self.value_number = Some(value);
        self
    }

    /// The control's value as a number, on a range or number input's
    /// `input` or `change`; `None` on an event that carries none (where
    /// the DOM's `valueAsNumber` would answer NaN).
    #[must_use]
    pub fn value_as_number(&self) -> Option<f64> {
        self.value_number
    }

    /// Attaches the `checked` payload.
    #[must_use]
    pub fn with_checked(mut self, checked: bool) -> Event {
        self.checked = Some(checked);
        self
    }

    /// Attaches where a pointer event happened.
    #[must_use]
    pub fn with_pointer(mut self, pointer: Pointer) -> Event {
        self.pointer = Some(pointer);
        self
    }

    /// The pointer's position from the viewport's top-left, CSS px;
    /// `[0.0, 0.0]` on an event that is not a pointer's.
    #[must_use]
    pub fn client(&self) -> [f32; 2] {
        self.pointer.map_or([0.0, 0.0], |pointer| pointer.client)
    }

    /// The pointer's position from the target's padding box, CSS px;
    /// `[0.0, 0.0]` on an event that is not a pointer's.
    #[must_use]
    pub fn offset(&self) -> [f32; 2] {
        self.pointer.map_or([0.0, 0.0], |pointer| pointer.offset)
    }

    /// Attaches the key of a `keydown` or `keyup`, as the DOM names it.
    #[must_use]
    pub fn with_key(mut self, key: &str) -> Event {
        self.key = Some(key.to_owned());
        self
    }

    /// The key of a `keydown` or `keyup` (`"Enter"`, `" "`, `"a"`), or
    /// empty on another event.
    #[must_use]
    pub fn key(&self) -> &str {
        self.key.as_deref().unwrap_or_default()
    }

    /// Names the node the event is dispatched to: the dispatcher sets it.
    #[must_use]
    pub fn at(mut self, target: NodeId) -> Event {
        self.target = Some(target);
        self
    }

    /// The node the event was dispatched to (the DOM's `target`), which
    /// stays the same as it bubbles; `None` before dispatch.
    #[must_use]
    pub fn target(&self) -> Option<NodeId> {
        self.target
    }

    /// The pointer's data, on a pointer event.
    #[must_use]
    pub fn pointer(&self) -> Option<Pointer> {
        self.pointer
    }

    /// From a `pointerdown` listener: sends this pointer's later events
    /// to the element whose listener this is, wherever the pointer goes,
    /// until `pointerup`. The web's `this.setPointerCapture(e.pointerId)`
    /// in a listener, with the listener's element standing for `this`.
    ///
    /// # Panics
    /// On an event with no pressed pointer (a `click`, a release, a key),
    /// as the web's `setPointerCapture` throws `InvalidStateError`: the
    /// capture would be dropped otherwise, with nothing said.
    pub fn set_pointer_capture(&mut self) {
        assert!(
            self.pointer.is_some_and(|pointer| pointer.pressed),
            "set_pointer_capture in a `{}` listener: there is no pressed pointer to capture; call it from pointerdown, or pointermove while pressed",
            self.name
        );
        self.capture = Some(Capture::Set);
    }

    /// Ends this pointer's capture: its events go to what is under it.
    pub fn release_pointer_capture(&mut self) {
        self.capture = Some(Capture::Release);
    }

    /// What the last listener asked of the capture, cleared as read, so
    /// the dispatcher knows which element asked.
    pub fn take_capture(&mut self) -> Option<Capture> {
        self.capture.take()
    }

    /// The kind of event: `click`, `input`, `change`, ...
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The control's value, or empty when the event carries none.
    #[must_use]
    pub fn value(&self) -> &str {
        self.value.as_deref().unwrap_or_default()
    }

    /// The control's checked state, on a checkbox's `input` or `change`;
    /// `None` on an event that carries none.
    #[must_use]
    pub fn checked(&self) -> Option<bool> {
        self.checked
    }

    /// Keeps the event from listeners further up the tree.
    pub fn stop_propagation(&mut self) {
        self.propagation_stopped = true;
    }

    /// Whether `stop_propagation` was called.
    #[must_use]
    pub fn propagation_stopped(&self) -> bool {
        self.propagation_stopped
    }

    /// Suppresses the built-in action that would follow.
    pub fn prevent_default(&mut self) {
        self.default_prevented = true;
    }

    /// Whether `prevent_default` was called.
    #[must_use]
    pub fn default_prevented(&self) -> bool {
        self.default_prevented
    }
}
