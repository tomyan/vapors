//! What a `keydown` or `keyup` carries, as the DOM's `KeyboardEvent`
//! names it: `key`, `code` and `repeat` (the modifiers are
//! [`Event::modifiers`], shared with pointer events).

use crate::Event;

impl Event {
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

    /// Attaches the key's place on the keyboard, as the DOM's
    /// `KeyboardEvent.code` names it (`"KeyH"`, `"Digit1"`).
    #[must_use]
    pub fn with_code(mut self, code: &str) -> Event {
        self.code = Some(code.to_owned());
        self
    }

    /// The key's place on the keyboard (`"KeyH"`, `"ShiftLeft"`), the
    /// same for its down and up whatever the modifiers do to `key`; empty
    /// on another event, or when the sender knows none.
    #[must_use]
    pub fn code(&self) -> &str {
        self.code.as_deref().unwrap_or_default()
    }

    /// Marks a `keydown` as a held key's auto-repeat.
    #[must_use]
    pub fn with_repeat(mut self, repeat: bool) -> Event {
        self.repeat = repeat;
        self
    }

    /// Whether this `keydown` is a held key's auto-repeat, as the DOM's
    /// `KeyboardEvent.repeat`; `false` on another event.
    #[must_use]
    pub fn repeat(&self) -> bool {
        self.repeat
    }
}
