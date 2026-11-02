//! The view's answers to input: scroll or pinch zooms about the pointer
//! or the fingers, a drag or a one-finger pan moves the image, a double
//! tap fits it or shows it actual size, the keys fit, show actual size
//! and zoom. A pointer gesture (a press with its drag and release, a
//! wheel, a touch with its pan, pinch and lift, a double tap) that starts
//! outside the area, or at a position the view does not know, is not
//! the view's: it moves nothing, and the view answers
//! [`Answered::OutsideArea`] for every event of it, so the host can give
//! the whole gesture to what it drew there (touché's tool card in the
//! side panel's inset).

use vapors_platform::{Event, LINE_PIXELS, TouchPhase};
use vapors_scene::Size;

use crate::ImageView;

/// Factor per scroll step or per `+`/`-` press.
const ZOOM_STEP: f32 = 1.25;

/// What the view did with an event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answered {
    /// The view answered it, or it means nothing to the view.
    Done,
    /// An event of a pointer gesture that started outside the area, or
    /// where the view does not know the pointer: nothing moved. The host
    /// gives it to its own drawing there; every event of the gesture,
    /// to its release or last lift, answers this.
    OutsideArea,
}

/// The gesture in progress.
#[derive(Default)]
pub(crate) struct Steering {
    /// The last pointer position; `None` until the pointer first moves.
    pointer: Option<[f32; 2]>,
    /// The primary button went down inside the area and is still down.
    dragging: bool,
    /// The primary button went down outside the area and is still down.
    pressed_outside: bool,
    /// The fingers down.
    touches: Vec<u64>,
    /// The first finger of the fingers down landed outside the area.
    touch_outside: bool,
}

impl ImageView {
    /// Answer an input event.
    pub fn event(&mut self, event: &Event) -> Answered {
        match event {
            Event::Resized { width, height } => self.resized(*width, *height),
            Event::PointerMoved { x, y } => return self.pointer_moved([*x, *y]),
            Event::PointerLeft => self.steering.pointer = None,
            Event::PrimaryButton { pressed, .. } => return self.pressed(*pressed),
            Event::Touch { id, phase, x, y } => return self.touched(*id, *phase, [*x, *y]),
            Event::Scroll { dy, .. } => return self.scrolled(*dy),
            Event::Pinch { x, y, factor } => return self.pinched([*x, *y], *factor),
            Event::Pan { dx, dy } => return self.panned(*dx, *dy),
            Event::DoubleTap { x, y } => return self.double_tapped([*x, *y]),
            Event::Character(text) => self.key(text),
            Event::Key { .. }
            | Event::Tap { .. }
            | Event::OpenRequested
            | Event::Paste
            | Event::FileOpened { .. } => {}
        }
        Answered::Done
    }

    fn resized(&mut self, width: u32, height: u32) {
        self.viewport = Size { width, height };
        self.fit();
    }

    fn pointer_moved(&mut self, to: [f32; 2]) -> Answered {
        if let (true, Some(from)) = (self.steering.dragging, self.steering.pointer) {
            self.pan(to[0] - from[0], to[1] - from[1]);
        }
        self.steering.pointer = Some(to);
        answer(self.steering.pressed_outside)
    }

    /// A press decides its gesture; its release answers as it did.
    fn pressed(&mut self, pressed: bool) -> Answered {
        let outside = if pressed {
            self.pointer_not_inside()
        } else {
            self.steering.pressed_outside
        };
        self.steering.dragging = pressed && !outside;
        self.steering.pressed_outside = pressed && outside;
        answer(outside)
    }

    /// The first finger decides its gesture, which the last finger's
    /// lift ends, answering as it did. A finger that starts while listed
    /// as down lost its end: the fingers listed are forgotten and it
    /// decides afresh.
    fn touched(&mut self, id: u64, phase: TouchPhase, at: [f32; 2]) -> Answered {
        let outside = self.touch_outside();
        let touches = &mut self.steering.touches;
        match phase {
            TouchPhase::Started if touches.contains(&id) || touches.is_empty() => {
                *touches = vec![id];
                self.steering.touch_outside = !self.area_contains(at);
                return answer(self.steering.touch_outside);
            }
            TouchPhase::Started => touches.push(id),
            TouchPhase::Moved => {}
            TouchPhase::Ended | TouchPhase::Cancelled => touches.retain(|t| *t != id),
        }
        answer(outside)
    }

    /// A notch away from the user (one line up) zooms in a step.
    fn scrolled(&mut self, dy: f32) -> Answered {
        if self.pointer_not_inside() {
            return Answered::OutsideArea;
        }
        self.zoom(self.scroll_anchor(), ZOOM_STEP.powf(-dy / LINE_PIXELS));
        Answered::Done
    }

    /// Fingers or a trackpad: the fingers' first landing decides, or,
    /// with no finger down (a trackpad), the anchor.
    fn pinched(&mut self, anchor: [f32; 2], factor: f32) -> Answered {
        let outside = if self.steering.touches.is_empty() {
            !self.area_contains(anchor)
        } else {
            self.touch_outside()
        };
        if !outside {
            self.zoom(anchor, factor);
        }
        answer(outside)
    }

    fn panned(&mut self, dx: f32, dy: f32) -> Answered {
        let outside = if self.steering.touches.is_empty() {
            self.pointer_not_inside()
        } else {
            self.touch_outside()
        };
        if !outside {
            self.pan(dx, dy);
        }
        answer(outside)
    }

    fn double_tapped(&mut self, at: [f32; 2]) -> Answered {
        let outside = !self.area_contains(at);
        if !outside {
            self.toggle_fit();
        }
        answer(outside)
    }

    fn touch_outside(&self) -> bool {
        !self.steering.touches.is_empty() && self.steering.touch_outside
    }

    /// The pointer is outside the area, or its position is not known:
    /// the view moves nothing on a position it does not know.
    fn pointer_not_inside(&self) -> bool {
        self.steering
            .pointer
            .is_none_or(|at| !self.area_contains(at))
    }

    /// The pointer: winit's wheel event carries no position of its own,
    /// and a wheel with none known is refused before this.
    fn scroll_anchor(&self) -> [f32; 2] {
        self.steering.pointer.unwrap_or_else(|| self.centre())
    }

    fn key(&mut self, text: &str) {
        match text {
            "0" => self.fit(),
            "1" => self.actual_size(),
            "+" | "=" => self.zoom(self.centre(), ZOOM_STEP),
            "-" => self.zoom(self.centre(), 1.0 / ZOOM_STEP),
            _ => {}
        }
    }
}

fn answer(outside: bool) -> Answered {
    if outside {
        Answered::OutsideArea
    } else {
        Answered::Done
    }
}

#[cfg(test)]
#[path = "image_view_input_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "image_view_gesture_tests.rs"]
mod gesture_tests;
