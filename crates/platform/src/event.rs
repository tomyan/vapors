//! Input the app receives between frames, in physical pixels.

/// One input event.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// The window's drawable area is now this size, in physical pixels.
    Resized {
        /// Width in pixels.
        width: u32,
        /// Height in pixels.
        height: u32,
    },
    /// The pointer moved to this position, in physical pixels from the top-left.
    PointerMoved {
        /// X in pixels.
        x: f32,
        /// Y in pixels.
        y: f32,
    },
    /// The pointer left the window; its position is unknown until it
    /// moves again.
    PointerLeft,
    /// The primary button went down or up.
    PrimaryButton {
        /// True on press.
        pressed: bool,
        /// The press's count in a row (two is a double click), counted by
        /// the platform (`clicks`) and carried on the press, so nothing
        /// waits for it; a release carries its press's.
        clicks: std::num::NonZeroU32,
    },
    /// The wheel or trackpad scrolled, in CSS pixels (not physical, unlike
    /// positions), signed as the DOM's `deltaX` and `deltaY`: positive
    /// scrolls right and down, moving the content left and up. A wheel
    /// notch is [`LINE_PIXELS`].
    Scroll {
        /// Rightward, CSS pixels.
        dx: f32,
        /// Downward, CSS pixels.
        dy: f32,
    },
    /// A key went down or up (3-E2a). Sent before the key's
    /// [`Event::Character`], as the DOM's `keydown` comes before its text.
    Key {
        /// The key as the DOM names it: the layout's character (`"a"`),
        /// or a named key's DOM name (`"Tab"`, `"Enter"`, `" "` for Space).
        key: String,
        /// True when it went down.
        pressed: bool,
        /// An auto-repeat of a held key.
        repeat: bool,
        /// The modifier keys held.
        modifiers: Modifiers,
    },
    /// A key produced this text on press.
    Character(String),
    /// Control+V, or Command+V on macOS: read the clipboard (a
    /// [`crate::Clipboard`]). Desktop only; a web page has its own paste
    /// event.
    Paste,
    /// The page's text input was edited (the web's hidden textarea,
    /// typing and an IME's composing alike): its whole state after the
    /// edit, and the text it was made against.
    TextEdited {
        /// The text the edit was made against: the last one mirrored.
        before: String,
        /// The text after it.
        text: String,
        /// The selection after it, start then end, in UTF-16 units.
        selection: (u32, u32),
        /// The range an IME is composing, in UTF-16 units, while it does.
        composing: Option<(u32, u32)>,
    },
    /// Text the page's paste delivered (the web's `paste` event on its
    /// text input), in place of [`Event::Paste`], which is the desktop's.
    PastedText(String),
    /// One finger on a touch screen, raw. Gestures are derived from these
    /// and sent as well.
    Touch {
        /// Which finger; stable from `Started` to `Ended` or `Cancelled`.
        id: u64,
        /// Where in the touch this event is.
        phase: TouchPhase,
        /// X in pixels.
        x: f32,
        /// Y in pixels.
        y: f32,
    },
    /// Two fingers moved apart or together, or a trackpad was pinched.
    /// Sent after the `Pan` of the same movement; apply in that order.
    Pinch {
        /// Anchor X in pixels: the point to zoom about.
        x: f32,
        /// Anchor Y in pixels.
        y: f32,
        /// Multiplicative change in spread since the last `Pinch`.
        factor: f32,
    },
    /// A finger, or the midpoint of two, moved by this much.
    Pan {
        /// Change in X, in pixels.
        dx: f32,
        /// Change in Y, in pixels.
        dy: f32,
    },
    /// A finger went down and up within 300 ms without moving 20 px.
    Tap {
        /// X in pixels where the finger lifted.
        x: f32,
        /// Y in pixels where the finger lifted.
        y: f32,
    },
    /// A second tap within 300 ms and 30 px of the first. Sent after
    /// that tap's own [`Event::Tap`].
    DoubleTap {
        /// X in pixels where the second finger lifted.
        x: f32,
        /// Y in pixels where the second finger lifted.
        y: f32,
    },
    /// The page's Open button was pressed: call
    /// [`Platform::pick_file`](crate::Platform::pick_file) with the kinds
    /// the app wants. Only the web sends it.
    OpenRequested,
    /// The file the user chose from the picker opened by
    /// [`Platform::pick_file`](crate::Platform::pick_file). Never sent on cancel.
    FileOpened {
        /// The file's name, without a directory.
        name: String,
        /// Its whole contents.
        bytes: Vec<u8>,
    },
}

/// Where in its life a touch is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TouchPhase {
    /// The finger went down.
    Started,
    /// The finger moved.
    Moved,
    /// The finger lifted.
    Ended,
    /// The system took the touch away; no `Ended` follows.
    Cancelled,
}

/// CSS pixels a wheel notch (a line) scrolls: what Chromium 155 scrolls
/// on Linux. Measured 2026-10-25 under Xvfb with one `XTest` button 5
/// press over a headful Chromium page (smooth scrolling off): its wheel
/// event had `deltaY` 120 and `deltaMode` 0 (pixels), and `scrollY`
/// moved by 120 per notch.
pub const LINE_PIXELS: f32 = 120.0;

/// The modifier keys held with a key.
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
