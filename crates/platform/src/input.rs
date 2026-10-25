//! Every winit window event becomes zero or more app events: the direct
//! translations from `event.rs`, plus the gestures recognised from touches
//! and trackpad pinches, which need state (the last pointer position and
//! the fingers down) that lives here.

use winit::event::{ElementState, WindowEvent};
use winit::keyboard::ModifiersState;

use crate::clock::Instant;
use crate::event::{Event, from_winit};
use crate::gestures::Gestures;

/// Stateful translation of winit events for one window.
pub struct Input {
    /// The pointer's last position; `None` before it moves and after it
    /// leaves.
    pointer: Option<[f32; 2]>,
    /// The window's size, from the last `Resized`.
    size: Option<[f32; 2]>,
    gestures: Gestures,
    started: Instant,
    /// Physical pixels per CSS pixel, for scroll deltas.
    scale: f64,
    /// The modifier keys held, for the paste shortcut.
    modifiers: ModifiersState,
}

impl Input {
    /// No fingers down, no pointer position and no size yet.
    pub fn new() -> Self {
        Self {
            pointer: None,
            size: None,
            gestures: Gestures::new(),
            started: Instant::now(),
            scale: 1.0,
            modifiers: ModifiersState::empty(),
        }
    }

    /// The window's physical pixels per CSS pixel, before each event.
    pub fn set_scale(&mut self, scale: f64) {
        self.scale = scale;
    }

    /// The app events for one winit event, in the order to send them.
    pub fn translate(&mut self, event: &WindowEvent) -> Vec<Event> {
        if let WindowEvent::PinchGesture { delta, .. } = event {
            return self.pinch(*delta).into_iter().collect();
        }
        if let Some(keys) = self.keys(event) {
            return keys;
        }
        let Some(direct) = from_winit(event, self.scale) else {
            return Vec::new();
        };
        let gestures = self.track(&direct);
        let mut events = vec![direct];
        events.extend(gestures);
        events
    }

    /// The modifiers kept, and the paste shortcut as [`Event::Paste`]
    /// instead of its character; `None` for other events. Desktop only.
    fn keys(&mut self, event: &WindowEvent) -> Option<Vec<Event>> {
        match event {
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
                Some(Vec::new())
            }
            WindowEvent::KeyboardInput { event: key, .. }
                if cfg!(not(target_family = "wasm"))
                    && key.state == ElementState::Pressed
                    && crate::paste_key::is_paste(
                        &key.logical_key,
                        self.modifiers,
                        cfg!(target_os = "macos"),
                    ) =>
            {
                Some(vec![Event::Paste])
            }
            _ => None,
        }
    }

    /// Updates what the gestures need from `event`; any gesture it ends.
    #[allow(clippy::cast_precision_loss)] // window sizes are far below 2^24
    fn track(&mut self, event: &Event) -> Vec<Event> {
        match event {
            Event::Resized { width, height } => self.size = Some([*width as f32, *height as f32]),
            Event::PointerMoved { x, y } => self.pointer = Some([*x, *y]),
            Event::PointerLeft => self.pointer = None,
            Event::Touch { id, phase, x, y } => {
                let now = self.started.elapsed();
                return self.gestures.touch(*id, *phase, *x, *y, now);
            }
            _ => {}
        }
        Vec::new()
    }

    /// A trackpad pinch, anchored at the pointer, else at the window's
    /// centre (winit's pinch carries no position); dropped before the
    /// window has a size, when there is nothing to zoom.
    #[allow(clippy::cast_possible_truncation)] // a trackpad delta is a small fraction
    fn pinch(&self, delta: f64) -> Option<Event> {
        let [x, y] = self
            .pointer
            .or_else(|| self.size.map(|[width, height]| [width / 2.0, height / 2.0]))?;
        Some(Event::Pinch {
            x,
            y,
            factor: 1.0 + delta as f32,
        })
    }
}

#[cfg(test)]
#[path = "input_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "input_wheel_tests.rs"]
mod wheel_tests;
