//! A winit window event as the app's [`Event`], in physical pixels:
//! what `Input` translates before it adds the gestures, click counts and
//! modifiers it keeps.

use winit::event::{ElementState, MouseScrollDelta, WindowEvent};

use crate::event::{Event, LINE_PIXELS, Modifiers, TouchPhase};

/// Translate a winit event into ours, the window at `scale` physical
/// pixels per CSS pixel. `None` for events the app does not get.
#[allow(clippy::cast_possible_truncation)] // positions and deltas are small
pub(crate) fn from_winit(event: &WindowEvent, scale: f64) -> Option<Event> {
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
            // counted by `Input`, which knows the last press
            clicks: std::num::NonZeroU32::MIN,
            // held, by `Input`, which keeps the modifiers
            modifiers: Modifiers::default(),
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

impl TouchPhase {
    pub(crate) fn from_winit(phase: winit::event::TouchPhase) -> Self {
        match phase {
            winit::event::TouchPhase::Started => Self::Started,
            winit::event::TouchPhase::Moved => Self::Moved,
            winit::event::TouchPhase::Ended => Self::Ended,
            winit::event::TouchPhase::Cancelled => Self::Cancelled,
        }
    }
}
