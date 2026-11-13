//! The platform `App` for a document: every frame updates it at the
//! window's size and scale and paints the result. A refused update or a
//! failed paint is reported (stderr natively, `#error` on the web), kept
//! for the caller in a [`Failure`], and stops the loop: the window never
//! shows a frame the document refused.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use vapors_elements::{Document, ShortcutModifier};
use vapors_platform::{App, Event, Frame, Next, Platform, Start, report_error};

use crate::document_input::{DocumentInput, Unplaced};
use crate::frame::FrameTimes;
use crate::painter::ScenePainter;
pub use decision::FrameDecision;
pub use failure::Failure;
pub use give::GiveError;
use give::Hosting;
pub use start::StartState;
pub use surface_handle::{InvalidateError, Invalidated, SurfaceHandle};
use surfaces::AppSurfaces;

/// Shows a document in the window. Events come through `App::event`,
/// or through [`ElementsApp::give`] for an app that wraps this one; one
/// instance takes them one way only (a debug build refuses the mix).
pub struct ElementsApp {
    document: Rc<RefCell<Document>>,
    painter: Option<ScenePainter>,
    /// The painter's atlas; `None` sizes it for the device.
    atlas: Option<vapors_text::AtlasConfig>,
    /// Surface renderers, before and after the painter exists.
    surfaces: AppSurfaces,
    failure: Failure,
    /// Input given to the document, at its last update's scale.
    input: DocumentInput,
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
    /// Why the last frame asked for the next (`elements_app_decision.rs`).
    decision: FrameDecision,
    /// Whether events came through `App::event` or `give`; `None` before the first.
    hosting: Option<Hosting>,
    /// Render passes the last painted frame's encoder began.
    render_passes: u64,
    /// How the painter came to be made: at `App::start` or not.
    start_state: StartState,
    /// Times frames on the GPU (`elements_app_timing.rs`).
    timer: AppTimer,
    /// The a11y mirror, when attached (`elements_app_a11y.rs`).
    a11y: a11y::A11yMirror,
}

/// What [`ElementsApp::with_per_frame`] calls before each update, with
/// the last frame's time per phase, by name, for the phases that do
/// work today (`elements_app_shown.rs`); empty before the first frame.
pub type PerFrame = Box<dyn FnMut(&[(&'static str, Duration)])>;

/// What [`ElementsApp::with_frame_times`] calls after each frame.
pub type FrameTimesHook = Box<dyn FnMut(&FrameTimes)>;

impl ElementsApp {
    /// Shows `document`, its shortcuts taking the platform's modifier
    /// (`vapors_platform::shortcut_is_command`).
    #[must_use]
    pub fn new(document: Rc<RefCell<Document>>) -> Self {
        // a listener's `Err` (`vapors_dom::ListenerResult`) is shown
        vapors_dom::set_error_report("vapors_platform::report_error", report_error);
        document
            .borrow_mut()
            .set_shortcut_modifier(shortcut_modifier());
        Self {
            document,
            painter: None,
            atlas: None,
            surfaces: AppSurfaces::default(),
            failure: Failure::default(),
            input: DocumentInput::new(),
            mirrored: None,
            per_frame: None,
            frame_times: None,
            last_times: None,
            input_spent: Duration::ZERO,
            decision: FrameDecision::default(),
            hosting: None,
            render_passes: 0,
            start_state: StartState::NotStarted,
            timer: AppTimer::default(),
            a11y: a11y::A11yMirror::default(),
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

/// Draws only when the document changed: an idle document draws no
/// frames.
impl App for ElementsApp {
    fn event(&mut self, event: Event, platform: &Platform) {
        match self.deliver(&event, platform, Hosting::Alone) {
            // Before the first frame nothing is on screen, so the event
            // was aimed at nothing; it is dropped.
            Ok(_) | Err(GiveError::Unplaced(Unplaced::NotUpdated)) => {}
            Err(GiveError::Unplaced(unplaced)) => {
                report_error(&unplaced.to_string());
                self.failure.set(&unplaced.to_string());
            }
            // refused text input is reported and the app goes on
            Err(refused) => report_error(&refused.to_string()),
        }
    }

    fn frame(&mut self, frame: &Frame<'_>) -> Next {
        #[cfg(not(target_family = "wasm"))]
        self.deliver_pastes();
        match self.run_phases(frame).and_then(|()| self.sync_animating()) {
            Ok(()) => self.next_after_frame(),
            Err(message) => {
                report_error(&message);
                self.failure.set(&message);
                self.surfaces.close();
                Next::Exit
            }
        }
    }

    fn start(&mut self, start: &Start<'_>) -> Result<(), String> {
        // the platform reports a refused start; the failure keeps it
        self.start_on(start)
            .inspect_err(|message| self.failure.set(message))
    }

    fn drawn(&mut self, platform: &Platform) {
        self.surfaces.keep_waker(platform);
        self.surfaces.give_waker(self.painter.as_mut());
        self.mirror_text_field(platform);
        self.hand_ax_update(platform);
    }
}

/// The modifier the platform's shortcuts take.
fn shortcut_modifier() -> ShortcutModifier {
    if vapors_platform::shortcut_is_command() {
        ShortcutModifier::Command
    } else {
        ShortcutModifier::Control
    }
}

#[path = "elements_app_a11y.rs"]
mod a11y;
#[path = "elements_app_atlas.rs"]
mod atlas;
#[path = "elements_app_counts.rs"]
mod counts;
#[path = "elements_app_decision.rs"]
mod decision;
#[path = "elements_app_failure.rs"]
mod failure;
#[path = "elements_app_frame.rs"]
mod frame_phases;
#[path = "elements_app_give.rs"]
mod give;
#[cfg(not(target_family = "wasm"))]
#[path = "elements_app_paste.rs"]
mod paste;
#[path = "elements_app_shown.rs"]
mod shown;
#[path = "elements_app_start.rs"]
mod start;
#[path = "elements_app_surface_handle.rs"]
mod surface_handle;
#[path = "elements_app_surfaces.rs"]
mod surfaces;
// For the root `gpu_` test module (the tier split needs a gpu_ id).
#[cfg(test)]
pub(crate) use surfaces::give_one_loop_waker_to;
#[cfg(test)]
#[path = "elements_app_tests.rs"]
mod tests;
#[path = "elements_app_text.rs"]
mod text;
#[path = "elements_app_timing.rs"]
mod timing;
use timing::AppTimer;
pub use timing::FrameTiming;
