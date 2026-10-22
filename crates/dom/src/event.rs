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
