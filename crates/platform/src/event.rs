//! Input the app receives between frames, in physical pixels.

use winit::event::{ElementState, MouseScrollDelta, WindowEvent};

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
    /// A key produced this text on press.
    Character(String),
    /// Control+V, or Command+V on macOS: read the clipboard (a
    /// [`crate::Clipboard`]). Desktop only; a web page has its own paste
    /// event.
    Paste,
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

impl TouchPhase {
    fn from_winit(phase: winit::event::TouchPhase) -> Self {
        match phase {
            winit::event::TouchPhase::Started => Self::Started,
            winit::event::TouchPhase::Moved => Self::Moved,
            winit::event::TouchPhase::Ended => Self::Ended,
            winit::event::TouchPhase::Cancelled => Self::Cancelled,
        }
    }
}

/// CSS pixels a wheel notch (a line) scrolls: what Chromium 155 scrolls
/// on Linux. Measured 2026-10-25 under Xvfb with one `XTest` button 5
/// press over a headful Chromium page (smooth scrolling off): its wheel
/// event had `deltaY` 120 and `deltaMode` 0 (pixels), and `scrollY`
/// moved by 120 per notch.
pub const LINE_PIXELS: f32 = 120.0;

/// Translate a winit event into ours, the window at `scale` physical
/// pixels per CSS pixel. `None` for events the app does not get.
#[allow(clippy::cast_possible_truncation)] // positions and deltas are small
pub fn from_winit(event: &WindowEvent, scale: f64) -> Option<Event> {
    match event {
        WindowEvent::Resized(size) => Some(Event::Resized {
            width: size.width,
            height: size.height,
        }),
        WindowEvent::CursorMoved { position, .. } => Some(Event::PointerMoved {
            x: position.x as f32,
            y: position.y as f32,
        }),
        WindowEvent::CursorLeft { .. } => Some(Event::PointerLeft),
        WindowEvent::MouseInput {
            state,
            button: winit::event::MouseButton::Left,
            ..
        } => Some(Event::PrimaryButton {
            pressed: *state == ElementState::Pressed,
        }),
        WindowEvent::MouseWheel { delta, .. } => Some(scroll(*delta, scale)),
        WindowEvent::Touch(touch) => Some(Event::Touch {
            id: touch.id,
            phase: TouchPhase::from_winit(touch.phase),
            x: touch.location.x as f32,
            y: touch.location.y as f32,
        }),
        WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => event
            .text
            .as_ref()
            .map(|text| Event::Character(text.to_string())),
        _ => None,
    }
}

/// winit's wheel delta as ours: winit negates the DOM's signs (its web
/// backend says so, `-event.delta_y()`), so they are negated back.
#[allow(clippy::cast_possible_truncation)] // deltas are small
fn scroll(delta: MouseScrollDelta, scale: f64) -> Event {
    let [dx, dy] = match delta {
        MouseScrollDelta::LineDelta(x, y) => [x * LINE_PIXELS, y * LINE_PIXELS],
        MouseScrollDelta::PixelDelta(at) => [(at.x / scale) as f32, (at.y / scale) as f32],
    };
    Event::Scroll { dx: -dx, dy: -dy }
}
