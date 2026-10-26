//! What an `.events` script does and checks (`events_script`): its
//! actions, in CSS px, and the expectations after each.

/// An input, in CSS px.
#[derive(Debug, PartialEq)]
pub enum Action {
    /// Press and release the left button.
    Click(f64, f64),
    /// A touch's start and end.
    Tap(f64, f64),
    /// Press the left button.
    Press(f64, f64),
    /// Release the left button.
    Release(f64, f64),
    /// Move the pointer.
    Move(f64, f64),
    /// One finger down.
    TouchDown(f64, f64),
    /// The finger moved.
    TouchMove(f64, f64),
    /// The finger lifted.
    TouchUp(f64, f64),
    /// A wheel turn at x, y by dx, dy.
    Wheel {
        /// Where, across.
        x: f64,
        /// Where, down.
        y: f64,
        /// How far across, positive to the right.
        dx: f64,
        /// How far down, positive down.
        dy: f64,
    },
    /// A key, with `+`-joined modifiers.
    Key(String),
    /// Text, one key per character.
    Type(String),
    /// Wait for the next painted frame.
    Frame,
}

/// What must hold after an action.
#[derive(Debug, PartialEq)]
pub enum Expect {
    /// An event of `kind` at `target` was logged.
    Event {
        /// The event type.
        kind: String,
        /// Where it was dispatched.
        target: String,
    },
    /// No event of this type was logged.
    NoEvent(String),
    /// This element has focus.
    Focus(String),
    /// This element has focus and matches `:focus-visible`.
    FocusVisible(String),
}
