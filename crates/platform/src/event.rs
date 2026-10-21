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
    /// The primary button went down or up.
    PrimaryButton {
        /// True on press.
        pressed: bool,
    },
    /// The wheel or trackpad scrolled. Positive `steps` is away from the user.
    Scroll {
        /// Scroll amount in notional lines; pixel deltas are converted at 50 px per line.
        steps: f32,
    },
    /// A key produced this text on press.
    Character(String),
}

/// Lines per pixel for pixel-delta scrolls. A first value; measured later
/// against a trackpad.
const PIXELS_PER_LINE: f32 = 50.0;

/// Translate a winit event into ours. `None` for events the app does not get.
#[allow(clippy::cast_possible_truncation)] // positions and deltas are small
pub fn from_winit(event: &WindowEvent) -> Option<Event> {
    match event {
        WindowEvent::Resized(size) => Some(Event::Resized {
            width: size.width,
            height: size.height,
        }),
        WindowEvent::CursorMoved { position, .. } => Some(Event::PointerMoved {
            x: position.x as f32,
            y: position.y as f32,
        }),
        WindowEvent::MouseInput {
            state,
            button: winit::event::MouseButton::Left,
            ..
        } => Some(Event::PrimaryButton {
            pressed: *state == ElementState::Pressed,
        }),
        WindowEvent::MouseWheel { delta, .. } => Some(Event::Scroll {
            steps: match delta {
                MouseScrollDelta::LineDelta(_, lines) => *lines,
                MouseScrollDelta::PixelDelta(position) => position.y as f32 / PIXELS_PER_LINE,
            },
        }),
        WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => event
            .text
            .as_ref()
            .map(|text| Event::Character(text.to_string())),
        _ => None,
    }
}
