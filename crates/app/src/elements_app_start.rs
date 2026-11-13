//! The app before its first frame (`App::start`, one-scene F0): its
//! painter made, every pipeline with it, its surface renderers mounted
//! and the glyph atlas's new page uploaded, so no shown frame pays for
//! them. The start's writes are submitted in it, so frame 0's submit
//! carries none of them.
//!
//! RELAXED: an app whose frames come without a start (tests that drive
//! `App::frame` alone, or a wrapping app that does not forward
//! `App::start`) makes the painter in its first frame, and says so on
//! the status line naming the fix; the judged run (`vapors-perf
//! --scene-run`) refuses that frame outright. Retired when every test
//! harness starts its app, so the first frame can refuse instead.

use vapors_platform::{Frame, Start, report_status};

use super::ElementsApp;
use crate::painter::ScenePainter;

/// How an app's painter, and every pipeline with it, came to be made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StartState {
    /// Neither `App::start` nor a frame has run.
    NotStarted,
    /// At `App::start`, before the first frame.
    Started,
    /// In the first frame, as no `App::start` came first: the status
    /// line said so, naming the fix.
    MadeInFirstFrame,
}

impl ElementsApp {
    /// How its painter came to be made: an `App` that wraps this one
    /// checks it is [`StartState::Started`] after its first frame.
    #[must_use]
    pub fn start_state(&self) -> StartState {
        self.start_state
    }

    /// [`App::start`](vapors_platform::App::start) for this app.
    pub(super) fn start_on(&mut self, start: &Start<'_>) -> Result<(), String> {
        if self.painter.is_some() {
            return Err(TWICE.to_owned());
        }
        let painter = self.make_painter(start.device, start.format);
        let painter = self
            .painter
            .insert(painter.map_err(|refused| refused.to_string())?);
        self.surfaces
            .bring_to(painter)
            .map_err(|error| format!("the surfaces could not be mounted: {error}"))?;
        painter
            .start(start.device, start.queue)
            .map_err(|error| format!("a surface could not be mounted: {error}"))?;
        // RELAXED: no test pins this submit. It runs the staged atlas and
        // mount writes now, not at the head of frame 0's submit (review of
        // #281, M2); wgpu exposes no pending-write count, so removing it
        // fails nothing but frame 0's measured time. Retired when wgpu can
        // report pending writes, or a GPU-time test resolves one copy.
        start.queue.submit([]);
        self.start_state = StartState::Started;
        Ok(())
    }

    /// The painter for the first frame of an app that was not started,
    /// with the status line saying so.
    pub(super) fn unstarted_painter(&self, frame: &Frame<'_>) -> Result<ScenePainter, String> {
        report_status(UNSTARTED);
        let made = self.make_painter(frame.device, frame.texture.format());
        made.map_err(|refused| refused.to_string())
    }

    /// Refused when the painter, made at start or by an earlier frame,
    /// draws into another format than `format`.
    pub(super) fn check_format(&self, format: wgpu::TextureFormat) -> Result<(), String> {
        match &self.painter {
            Some(painter) if painter.format() != format => Err(format!(
                "the frame's texture is {format:?} but the app started for {:?}; fix: \
                 give App::start the format every frame's texture has",
                painter.format()
            )),
            _ => Ok(()),
        }
    }
}

/// `App::start` came after the painter was made.
const TWICE: &str = "App::start was called after the app's painter was made (a second start, \
    or a start after a frame); fix: call App::start once, before the first frame";

/// A frame came before any `App::start`.
const UNSTARTED: &str = "the first frame made the painter and built its pipelines: App::start \
    was not called; fix: call App::start before the first frame (an App wrapping an ElementsApp \
    forwards it)";
