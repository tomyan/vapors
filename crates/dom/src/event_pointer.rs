//! A pointer event's place and pointer, and what a listener asks of its
//! capture.

/// Where a pointer event happened, and which pointer: positions in CSS
/// px, unrounded.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pointer {
    /// From the viewport's top-left.
    pub client: [f32; 2],
    /// From the target's padding box's top-left.
    pub offset: [f32; 2],
    /// 1 for the mouse; each touch has its own (3-E1b).
    pub id: u32,
    /// What made it, as the web's `PointerEvent.pointerType`. Set by
    /// whoever makes the pointer from what its source says; there is no
    /// default, so a constructor that cannot tell must refuse (naming
    /// its file and line), not pick `Mouse`.
    pub kind: PointerKind,
    /// Whether a button or finger is down for this event, so a listener
    /// may capture the pointer: on `pointerdown` and on `pointermove`
    /// while pressed.
    pub pressed: bool,
}

/// The device behind a pointer: the web's `pointerType` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PointerKind {
    /// A mouse, or a trackpad acting as one.
    Mouse,
    /// A finger on a touch screen.
    Touch,
    /// A pen or stylus. winit 0.30 does not report one on desktop; a
    /// source that can tell sets it.
    Pen,
}

impl PointerKind {
    /// The web's `pointerType` string for it.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mouse => "mouse",
            Self::Touch => "touch",
            Self::Pen => "pen",
        }
    }
}

/// What a listener asked of the pointer's capture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capture {
    /// `set_pointer_capture`.
    Set,
    /// `release_pointer_capture`.
    Release,
}
