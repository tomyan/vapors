//! The event a listener receives.

/// An event delivered to a listener. The payload fields are set by
/// the kind of event: `input` and `change` carry `value`, `change` on
/// a checkbox carries `checked`.
#[derive(Debug, Clone, Default)]
pub struct Event {
    name: String,
    value: Option<String>,
    checked: Option<bool>,
    propagation_stopped: bool,
    default_prevented: bool,
    pointer: Option<Pointer>,
    capture: Option<Capture>,
}

/// Where a pointer event happened, and which pointer: positions in CSS
/// px, unrounded.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Pointer {
    /// From the viewport's top-left.
    pub client: [f32; 2],
    /// From the target's padding box's top-left.
    pub offset: [f32; 2],
    /// 1 for the mouse; each touch has its own (3-E1b).
    pub id: u32,
}

/// What a listener asked of the pointer's capture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capture {
    /// `set_pointer_capture`.
    Set,
    /// `release_pointer_capture`.
    Release,
}

/// A listener: boxed so the tree can store it.
pub type Handler = Box<dyn FnMut(&mut Event)>;

impl Event {
    /// An event of the given kind with no payload.
    #[must_use]
    pub fn new(name: &str) -> Event {
        Event {
            name: name.to_string(),
            ..Event::default()
        }
    }

    /// Attaches the `value` payload.
    #[must_use]
    pub fn with_value(mut self, value: &str) -> Event {
        self.value = Some(value.to_string());
        self
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

    /// The pointer's data, on a pointer event.
    #[must_use]
    pub fn pointer(&self) -> Option<Pointer> {
        self.pointer
    }

    /// From a `pointerdown` listener: sends this pointer's later events
    /// to the element whose listener this is, wherever the pointer goes,
    /// until `pointerup`. The web's `this.setPointerCapture(e.pointerId)`
    /// in a listener, with the listener's element standing for `this`.
    pub fn set_pointer_capture(&mut self) {
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

    /// The control's checked state, or `false` when the event carries none.
    #[must_use]
    pub fn checked(&self) -> bool {
        self.checked.unwrap_or(false)
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
