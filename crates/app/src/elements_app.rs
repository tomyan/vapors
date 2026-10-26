//! The platform `App` for a document: every frame updates it at the
//! window's size and scale and paints the result. A refused update or a
//! failed paint is reported (stderr natively, `#error` on the web), kept
//! for the caller in a [`Failure`], and stops the loop: the window never
//! shows a frame the document refused.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use vapors_elements::{Cursor, Document, ElementError, Point, Viewport};
use vapors_platform::{App, Event, Frame, Instant, Next, Platform, report_error};

use crate::frame::FrameTimes;
use vapors_scene::Size;

use crate::painter::ScenePainter;
use crate::pointer::action;

/// Why the app stopped, once it has: shared with whoever ran it.
#[derive(Clone, Default)]
pub struct Failure(Rc<RefCell<Option<String>>>);

impl Failure {
    /// The report, if the app stopped on a failure.
    #[must_use]
    pub fn get(&self) -> Option<String> {
        self.0.borrow().clone()
    }

    fn set(&self, message: &str) {
        *self.0.borrow_mut() = Some(message.to_owned());
    }
}

/// Shows a document in the window.
pub struct ElementsApp {
    document: Rc<RefCell<Document>>,
    painter: Option<ScenePainter>,
    failure: Failure,
    /// The last frame's display scale, for input in physical pixels.
    scale: f32,
    /// Where the pointer last moved, in CSS pixels.
    pointer: Point,
    touch: crate::touch_scroll::TouchScroll,
    wheel: crate::wheel_scroll::WheelScroll,
    /// The cursor last shown.
    cursor: Cursor,
    /// Called before each update with the last frame's phase times.
    per_frame: Option<PerFrame>,
    /// Called after each frame with its phase times.
    frame_times: Option<FrameTimesHook>,
    /// The last frame's phase times; `None` before the first.
    last_times: Option<FrameTimes>,
    /// Time spent handling events since the last frame: the Input phase.
    input_spent: Duration,
}

/// What [`ElementsApp::with_per_frame`] calls before each update, with
/// the last frame's time per phase, by name, for the phases that do
/// work today (input, update, draw); empty before the first frame.
pub type PerFrame = Box<dyn FnMut(&[(&'static str, Duration)])>;

/// What [`ElementsApp::with_frame_times`] calls after each frame.
pub type FrameTimesHook = Box<dyn FnMut(&FrameTimes)>;

impl ElementsApp {
    /// Shows `document`.
    #[must_use]
    pub fn new(document: Rc<RefCell<Document>>) -> Self {
        Self {
            document,
            painter: None,
            failure: Failure::default(),
            scale: 1.0,
            pointer: [0.0, 0.0],
            touch: crate::touch_scroll::TouchScroll::default(),
            wheel: crate::wheel_scroll::WheelScroll::default(),
            cursor: Cursor::Default,
            per_frame: None,
            frame_times: None,
            last_times: None,
            input_spent: Duration::ZERO,
        }
    }

    /// Calls `per_frame`, when there is one, before each update with the
    /// last frame's phase times: for a demo that changes the document
    /// every frame and shows its own frame time (`stress`). What it
    /// changes asks for the next frame, as any change does.
    #[must_use]
    pub fn with_per_frame(mut self, per_frame: Option<PerFrame>) -> Self {
        self.per_frame = per_frame;
        self
    }

    /// Calls `frame_times` after each frame with its phase times (2-S1):
    /// what an app records as its frame numbers (touché's P0).
    #[must_use]
    pub fn with_frame_times(mut self, frame_times: FrameTimesHook) -> Self {
        self.frame_times = Some(frame_times);
        self
    }

    /// Takes input as if the display scale were `scale`, until the first
    /// frame says what it is: for driving `event` without a window.
    #[must_use]
    pub fn with_scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    /// Where the app's failure is kept; clone it before `run` takes the app.
    #[must_use]
    pub fn failure(&self) -> Failure {
        self.failure.clone()
    }
}

impl ElementsApp {
    /// What to do after a frame: draw again at once if the document
    /// changed since it was last asked (by something outside an event,
    /// or by the update itself), else wait for the next change.
    pub fn next_after_frame(&mut self) -> Next {
        if self.document.borrow_mut().take_changed() {
            Next::Continue
        } else {
            Next::Wait
        }
    }
}

/// Draws only when the document changed: an idle document draws no
/// frames.
impl App for ElementsApp {
    fn event(&mut self, event: Event, platform: &Platform) {
        let started = Instant::now();
        self.handle(&event, platform);
        self.input_spent += started.elapsed();
    }

    fn frame(&mut self, frame: &Frame<'_>) -> Next {
        match self.run_phases(frame) {
            Ok(()) => self.next_after_frame(),
            Err(message) => {
                report_error(&message);
                self.failure.set(&message);
                Next::Exit
            }
        }
    }
}

impl ElementsApp {
    /// An event, dispatched as it arrives (design 02, 2-S1).
    fn handle(&mut self, event: &Event, platform: &Platform) {
        if let Some(action) = action(event, self.scale, self.pointer) {
            self.act(action);
        }
        self.key(event);
        if self.document.borrow_mut().take_changed() {
            platform.request_redraw();
        }
        let cursor = self.document.borrow().cursor(self.pointer);
        if cursor != self.cursor {
            self.cursor = cursor;
            platform.set_cursor(crate::cursor::shown(cursor));
        }
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)] // window sizes and scales are small
fn viewport(size: Size, scale: f32) -> Viewport {
    Viewport {
        width: size.width as f32 / scale,
        height: size.height as f32 / scale,
        scale,
    }
}

fn size_of(frame: &Frame<'_>) -> Size {
    Size {
        width: frame.texture.width(),
        height: frame.texture.height(),
    }
}

fn refused(errors: &[ElementError]) -> String {
    let lines: Vec<String> = errors.iter().map(|error| format!("  {error}")).collect();
    format!(
        "the document was refused, so nothing is drawn:\n{}",
        lines.join("\n")
    )
}

#[path = "elements_app_act.rs"]
mod act;
#[path = "elements_app_frame.rs"]
mod frame_phases;
