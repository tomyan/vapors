//! A pointer event's place and pointer, and what a listener asks of its
//! capture.

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
    /// Whether a button or finger is down for this event, so a listener
    /// may capture the pointer: on `pointerdown` and on `pointermove`
    /// while pressed.
    pub pressed: bool,
}

/// What a listener asked of the pointer's capture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capture {
    /// `set_pointer_capture`.
    Set,
    /// `release_pointer_capture`.
    Release,
}
