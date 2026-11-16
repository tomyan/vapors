//! Every winit window event becomes zero or more app events: the direct
//! translations from `event.rs`, plus the gestures recognised from touches
//! and trackpad pinches, which need state (the last pointer position and
//! the fingers down) that lives here.

use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::keyboard::ModifiersState;

use crate::paste_key::Press;

use crate::clock::Instant;
use crate::event::Event;
use crate::event_winit::from_winit;
use crate::gestures::Gestures;
pub(crate) use crate::input_keys::Key;
use crate::input_keys::pressed;

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
    /// The presses in a row.
    clicks: crate::clicks::Clicks,
}

impl Input {
    /// No fingers down, no pointer position and no size yet.
    pub fn new() -> Self {
        let started = Instant::now();
        Self {
            pointer: None,
            size: None,
            gestures: Gestures::starting(started),
            started,
            scale: 1.0,
            modifiers: ModifiersState::empty(),
            clicks: crate::clicks::Clicks::default(),
        }
    }

    /// The window's physical pixels per CSS pixel, before each event.
    pub fn set_scale(&mut self, scale: f64) {
        self.scale = scale;
    }

    /// [`Input::translate`] at the system clock's time, for the tests
    /// whose events carry no time of their own.
    #[cfg(test)]
    pub(crate) fn translate_now(&mut self, event: &WindowEvent) -> Vec<Event> {
        self.translate(event, Instant::now())
    }

    /// The app events for one winit event taken at `now` (the event
    /// loop's one read of the clock for it), in the order to send them.
    pub fn translate(&mut self, event: &WindowEvent, now: Instant) -> Vec<Event> {
        if let WindowEvent::PinchGesture { delta, .. } = event {
            return self.pinch(*delta).into_iter().collect();
        }
        if let Some(keys) = self.keys(event) {
            return keys;
        }
        self.break_row_on_other_buttons(event);
        let Some(direct) = from_winit(event, self.scale) else {
            return Vec::new();
        };
        let direct = self.counted(direct, now);
        let gestures = self.track(&direct, now);
        let mut events: Vec<Event> = self.wheel_placement(event, now).into_iter().collect();
        events.push(direct);
        events.extend(gestures);
        events
    }

    /// A press of a button other than the primary is no app event, but
    /// breaks the primary's row, as Chromium counts presses of one
    /// button in a row (review-v100, S1).
    fn break_row_on_other_buttons(&mut self, event: &WindowEvent) {
        if let WindowEvent::MouseInput {
            state: ElementState::Pressed,
            button,
            ..
        } = event
            && *button != MouseButton::Left
        {
            self.clicks.break_row();
        }
    }

    /// The modifiers kept, and each key press as its [`Event::Key`] then
    /// its paste or character; `None` for other events.
    fn keys(&mut self, event: &WindowEvent) -> Option<Vec<Event>> {
        let key = match event {
            WindowEvent::ModifiersChanged(modifiers) => Key::Modifiers(modifiers.state()),
            WindowEvent::Focused(focused) => Key::Focus(*focused),
            WindowEvent::KeyboardInput { event: key, .. } => Key::Press(Press {
                state: key.state,
                repeat: key.repeat,
                logical: &key.logical_key,
                physical: key.physical_key,
                modifiers: self.modifiers,
                text: key.text.as_deref(),
            }),
            _ => return None,
        };
        Some(self.key(key))
    }

    /// One keyboard fact: the modifiers change; the window's focus
    /// changes, sent as [`Event::WindowFocus`] (lost, no modifier is known
    /// held: some backends send no `ModifiersChanged` then); or a key is
    /// pressed, as its `Key` then its paste or character.
    ///
    /// A paste goes to the app whatever has focus, its character
    /// swallowed: focus lives in the app's document, so the app hands
    /// it to a focused text field first (`ElementsApp`, E6b).
    pub(crate) fn key(&mut self, key: Key<'_>) -> Vec<Event> {
        match key {
            Key::Modifiers(modifiers) => {
                self.modifiers = modifiers;
                Vec::new()
            }
            Key::Focus(focused) => {
                if !focused {
                    self.modifiers = ModifiersState::empty();
                }
                vec![Event::WindowFocus { focused }]
            }
            Key::Press(press) => pressed(&Press {
                modifiers: self.modifiers,
                ..press
            }),
        }
    }

    /// Updates what the gestures need from `event`; any gesture it ends.
    #[allow(clippy::cast_precision_loss)] // window sizes are far below 2^24
    fn track(&mut self, event: &Event, now: Instant) -> Vec<Event> {
        match event {
            Event::Resized { width, height } => self.size = Some([*width as f32, *height as f32]),
            Event::PointerMoved { x, y } => self.pointer = Some([*x, *y]),
            Event::PointerLeft => self.pointer = None,
            Event::Touch { id, phase, x, y } => {
                // RELAXED: the time this touch is handled, not the touch's
                // own (winit 0.30 gives none, rust-windowing/winit#1194).
                // The velocity tracker's 40 ms and 80 ms stop rules then
                // measure handling gaps as well as a stopped finger
                // (`velocity_tracker.rs`). Retired by #403.
                let since = now.saturating_duration_since(self.started);
                return self.gestures.touch(*id, *phase, *x, *y, since);
            }
            _ => {}
        }
        Vec::new()
    }

    /// A press of the primary button with its count in a row at the
    /// pointer (CSS px), stamped now; a release with its press's count;
    /// any other event as it is. A press with no known pointer position
    /// starts a new row.
    ///
    /// RELAXED: winit 0.30 gives events no timestamp, so a press is
    /// stamped when winit hands it over; presses queued behind a slow
    /// frame are stamped together and can count as a row. Retired when
    /// winit exposes event times.
    #[allow(clippy::cast_possible_truncation)] // a display scale
    fn counted(&mut self, event: Event, now: Instant) -> Event {
        let Event::PrimaryButton { pressed, .. } = event else {
            return event;
        };
        let scale = self.scale as f32;
        let at = self.pointer.map(|[x, y]| [x / scale, y / scale]);
        let clicks = if pressed {
            self.clicks.pressed(at, now)
        } else {
            self.clicks.last()
        };
        let modifiers = crate::key_event::modifiers(self.modifiers);
        Event::PrimaryButton {
            pressed,
            clicks,
            modifiers,
        }
    }
}

#[path = "input_pointer.rs"]
mod pointer;

#[cfg(test)]
#[path = "input_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "input_clock_tests.rs"]
mod clock_tests;

#[cfg(test)]
#[path = "input_click_tests.rs"]
mod click_tests;

#[cfg(test)]
#[path = "input_wheel_tests.rs"]
mod wheel_tests;

#[cfg(test)]
#[path = "input_key_tests.rs"]
mod key_tests;

#[cfg(test)]
#[path = "key_event_tests.rs"]
mod key_event_tests;
