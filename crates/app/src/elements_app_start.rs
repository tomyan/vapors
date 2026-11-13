//! The app before its first frame (`App::start`, one-scene F0): its
//! painter made, every pipeline with it, its surface renderers mounted,
//! the glyph atlas's new page uploaded, the frame timer made, and the
//! document's first update at the window's size with each surface's
//! backing made at its laid-out size, so no shown frame pays for them.
//! The start's writes are submitted in it, so frame 0's submit carries
//! none of them.
//!
//! RELAXED: an app whose frames come without a start (tests that drive
//! `App::frame` alone, or a wrapping app that does not forward
//! `App::start`) makes the painter in its first frame, and says so on
//! the status line naming the fix; the judged run (`vapors-perf
//! --scene-run`) refuses that frame outright. Retired when every test
//! harness starts its app, so the first frame can refuse instead.

use vapors_platform::{Frame, Start, report_status};
use vapors_scene::Size;

use super::ElementsApp;
use crate::painter::ScenePainter;

/// How an app's painter, and every pipeline with it, came to be made.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StartState {
    /// Neither `App::start` nor a frame has run.
    NotStarted,
    /// At `App::start`, before the first frame.
    Started,
    /// In the first frame, as no `App::start` came first: the status
    /// line said so, naming the fix.
    MadeInFirstFrame,
    /// At `App::start`, but frame 0 came at another size or scale: its
    /// surfaces' backings were remade in frame 0, and the status line
    /// said so.
    StartedAtOtherSize {
        /// The start's size in device pixels, and its scale.
        start: (Size, f32),
        /// Frame 0's size in device pixels, and its scale.
        frame: (Size, f32),
    },
}

impl StartState {
    /// Whether `App::start` ran before the first frame.
    #[must_use]
    pub fn started(self) -> bool {
        matches!(self, Self::Started | Self::StartedAtOtherSize { .. })
    }
}

/// How the app started, and the start's target until frame 0 is checked
/// against it.
pub(super) struct StartRecord {
    pub(super) state: StartState,
    target: Option<(Size, f32)>,
}

impl Default for StartRecord {
    fn default() -> Self {
        Self {
            state: StartState::NotStarted,
            target: None,
        }
    }
}

impl ElementsApp {
    /// How its painter came to be made: an `App` that wraps this one
    /// checks [`StartState::started`] after its first frame.
    #[must_use]
    pub fn start_state(&self) -> StartState {
        self.start.state
    }

    /// At the first frame after a start: a frame at another size or
    /// scale than the start's remakes the surfaces' backings, which the
    /// start was to spare it, so it is said on the status line and kept
    /// in the state (review of #303, M1).
    pub(super) fn check_start_target(&mut self, size: Size, scale: f32) {
        let Some((start, start_scale)) = self.start.target.take() else {
            return;
        };
        #[allow(clippy::float_cmp)] // the same window's scale, compared exactly
        if start == size && start_scale == scale {
            return;
        }
        report_status(&format!(
            "frame 0 is {}x{} at scale {scale} but App::start was {}x{} at scale \
             {start_scale}: the surfaces' backings were remade in frame 0",
            size.width, size.height, start.width, start.height
        ));
        self.start.state = StartState::StartedAtOtherSize {
            start: (start, start_scale),
            frame: (size, scale),
        };
    }

    /// [`App::start`](vapors_platform::App::start) for this app.
    pub(super) fn start_on(&mut self, start: &Start<'_>) -> Result<(), String> {
        if self.painter.is_some() {
            return Err(TWICE.to_owned());
        }
        let size = start_size(start)?;
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
        // RELAXED: the timer made here is pinned (gpu_start_tests.rs); its
        // order before the backings' clear (Mounted::start_at) is not. Measured in diagnostic
        // builds on a loaded box (1-minute load 20 to 60), not acceptance
        // runs: frame 0's surface submit took 13 to 20 ms unless both the
        // timer and the clear came at start, in this order, and 0.2 to 0.4
        // ms when they did (one-scene.md, F0). Retired by the acceptance
        // runs, or a GPU-time test of frame 0's submit.
        self.timer.start(start.device, start.queue);
        self.first_update(start, size)?;
        // RELAXED: no test pins this submit. It runs the staged atlas and
        // mount writes now, not at the head of frame 0's submit (review of
        // #281, M2); wgpu exposes no pending-write count, so removing it
        // fails nothing but frame 0's measured time. Retired when wgpu can
        // report pending writes, or a GPU-time test resolves one copy.
        start.queue.submit([]);
        self.start.state = StartState::Started;
        #[allow(clippy::cast_possible_truncation)] // a display scale
        let scale = start.scale_factor as f32;
        self.start.target = Some((size, scale));
        Ok(())
    }

    /// The document's first update at the start's size, then each of its
    /// surfaces' backings made at its laid-out size.
    fn first_update(&mut self, start: &Start<'_>, size: Size) -> Result<(), String> {
        let max_side = start.device.limits().max_texture_dimension_2d;
        #[allow(clippy::cast_possible_truncation)] // a display scale
        self.update_document_at(max_side, size, start.scale_factor as f32)?;
        let document = self.document.borrow();
        let scene = document.painted().expect("updated above");
        let painter = self.painter.as_mut().expect("made by start_on");
        painter
            .start_surfaces(start.device, start.queue, scene)
            .map_err(|error| format!("a surface could not be sized at start: {error}"))
    }

    /// The painter for the first frame of an app that was not started,
    /// with the status line saying so.
    pub(super) fn unstarted_painter(&self, frame: &Frame<'_>) -> Result<ScenePainter, String> {
        // RELAXED: the state beside it is pinned (gpu_app_start.rs); this
        // line is not, as the platform's report has no test sink. Retired
        // when report_status can be captured in a test.
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

/// The start's size; refused when it has no pixels, since the first
/// update lays the document out at it.
fn start_size(start: &Start<'_>) -> Result<Size, String> {
    if start.width == 0 || start.height == 0 {
        return Err(format!(
            "App::start was given a {}x{} surface, so the document cannot be laid out \
             before the first frame; fix: start the app once the window has its size",
            start.width, start.height
        ));
    }
    Ok(Size {
        width: start.width,
        height: start.height,
    })
}

/// `App::start` came after the painter was made.
const TWICE: &str = "App::start was called after the app's painter was made (a second start, \
    or a start after a frame); fix: call App::start once, before the first frame";

/// A frame came before any `App::start`.
const UNSTARTED: &str = "the first frame made the painter and built its pipelines: App::start \
    was not called; fix: call App::start before the first frame (an App wrapping an ElementsApp \
    forwards it)";
