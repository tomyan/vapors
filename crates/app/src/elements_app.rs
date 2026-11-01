//! The platform `App` for a document: every frame updates it at the
//! window's size and scale and paints the result. A refused update or a
//! failed paint is reported (stderr natively, `#error` on the web), kept
//! for the caller in a [`Failure`], and stops the loop: the window never
//! shows a frame the document refused.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use vapors_elements::Document;
use vapors_platform::{App, Event, Frame, Instant, Next, Platform, report_error};

use crate::frame::FrameTimes;

use crate::document_input::DocumentInput;
use crate::painter::ScenePainter;

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
    /// Input given to the document, at its last update's scale.
    input: DocumentInput,
    /// The clipboard a paste into a text field reads (E6b).
    #[cfg(not(target_family = "wasm"))]
    clipboard: Box<dyn vapors_platform::Clipboard>,
    /// The clipboard reads pastes started, answered off the loop.
    #[cfg(not(target_family = "wasm"))]
    pastes: paste::Pastes,
    /// The text field last set for the platform; `None` before the
    /// first, or when it must be set again.
    // set to no field (`Some(None)`) differs from never set (`None`)
    #[allow(clippy::option_option)]
    mirrored: Option<Option<vapors_platform::TextField>>,
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
        // a listener's `Err` (`vapors_dom::ListenerResult`) is shown
        vapors_dom::set_error_report("vapors_platform::report_error", report_error);
        Self {
            document,
            painter: None,
            failure: Failure::default(),
            input: DocumentInput::new(),
            #[cfg(not(target_family = "wasm"))]
            clipboard: Box::new(vapors_platform::SystemClipboard::default()),
            #[cfg(not(target_family = "wasm"))]
            pastes: paste::Pastes::default(),
            mirrored: None,
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
        #[cfg(not(target_family = "wasm"))]
        self.deliver_pastes();
        match self.run_phases(frame) {
            Ok(()) => self.next_after_frame(),
            Err(message) => {
                report_error(&message);
                self.failure.set(&message);
                Next::Exit
            }
        }
    }

    fn drawn(&mut self, platform: &Platform) {
        self.mirror_text_field(platform);
    }
}

impl ElementsApp {
    /// An event, dispatched as it arrives (design 02, 2-S1).
    fn handle(&mut self, event: &Event, platform: &Platform) {
        match self.input.handle(&self.document, event) {
            Ok(outcome) => {
                if outcome.changed {
                    platform.request_redraw();
                }
                if let Some(cursor) = outcome.cursor {
                    platform.set_cursor(cursor);
                }
            }
            // Before the first frame nothing is on screen, so the event
            // was aimed at nothing; it is dropped.
            Err(crate::document_input::Unplaced::NotUpdated) => {}
            Err(unplaced) => {
                report_error(&unplaced.to_string());
                self.failure.set(&unplaced.to_string());
            }
        }
        self.text_event(event);
        #[cfg(not(target_family = "wasm"))]
        if matches!(event, Event::Paste) {
            self.paste(platform);
        }
        if self.document.borrow_mut().take_changed() {
            platform.request_redraw();
        }
        self.mirror_text_field(platform);
    }
}

#[path = "elements_app_frame.rs"]
mod frame_phases;
#[cfg(not(target_family = "wasm"))]
#[path = "elements_app_paste.rs"]
mod paste;
#[path = "elements_app_text.rs"]
mod text;
