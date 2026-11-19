//! A renderer's own GPU errors, caught by a validation error scope
//! around each call into it and refused as its failure, naming the
//! surface, rather than panicking unnamed in wgpu's uncaptured-error
//! handler. Native wgpu resolves a scope when it is popped. On the web
//! the result arrives later: an unresolved scope is watched against a
//! timer at its deadline (`painter_surface_scope_watch.rs`), and an
//! error or the deadline wakes the loop for a frame that refuses the
//! surface; a clean result wakes nothing and clears the timer. It asks
//! for no frame while it waits.

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use vapors_platform::Instant;

use super::super::errors::Refusal;
use crate::surface::SurfaceError;

#[cfg(any(target_family = "wasm", test))]
#[path = "painter_surface_scope_deadline.rs"]
mod deadline;
#[path = "painter_surface_scope_watch.rs"]
mod watch;
pub(super) use watch::LoopWaker;
use watch::{Outcome, Watched};

/// The GPU's report that the work a scope covers is done.
pub(super) type WorkDone = Pin<Box<dyn Future<Output = ()>>>;

/// A scope's result, once the GPU reports it.
pub(super) type Caught = Pin<Box<dyn Future<Output = Option<wgpu::Error>>>>;

/// How long a scope may stay unresolved before its surface is refused.
///
/// Measured 2026-11-10: headless Chromium 155.0.8059.79, WebGPU on
/// `SwiftShader` (Vulkan), load average 5 to 16 on 12 cores.
/// `popErrorScope` around a render pass and submit, once per animation
/// frame, resolved in p50 0.4 to 0.9 ms, p99 6 to 12 ms, max 37.6 ms
/// over 6,500 renders; around a call that makes a validation error, max
/// 8 ms over 650. Rerun: `PLAYWRIGHT_MODULE=<playwright-core/index.mjs>
/// node scripts/web-scope-latency.mjs [renders]`.
///
/// 1000 ms is about 26 times the largest. 100 ms would be under 3
/// times it, and the largest came from a loaded machine, where a stall
/// of the browser's GPU process can take longer still; a slow phone or
/// a heavier render adds more. Refusing a working surface is the worse
/// error, and a scope that never resolves is refused within a second
/// either way. Native wgpu resolves in the frame, so it never waits.
///
/// On the web only time with the page visible counts: a hidden page
/// draws no frames, throttles its timers, and its `popErrorScope`
/// latency is unmeasured. A period the page was hidden at either end is
/// re-armed in full (`visible_deadline`), since refusing is permanent
/// until remount.
///
/// On the web, for a scope around work submitted to the queue, it runs
/// from when the GPU reports that work done (`WORK_DONE_DEADLINE` bounds
/// that wait), not from when the scope is popped: it measures the
/// browser reporting an error, not the GPU drawing.
pub(crate) const DEADLINE: Duration = Duration::from_millis(1000);

/// How long, on the web, the GPU may take to report the work a scope
/// covers done before its surface is refused. `DEADLINE` runs from that
/// report, so this bounds the drawing and that bounds the reporting.
///
/// A GUESS, not a measurement: no browser runs in the build sandbox.
/// The Toad Trip lit frame (1280x720, sun and two point lights, headless
/// Chromium on `SwiftShader`) took over `DEADLINE` and never finished
/// inside a second. 30 s is generous for a slow GPU, and still ends a
/// hung one with a named refusal. To measure: time `submit` to
/// `onSubmittedWorkDone` for that page and for a heavier one, as
/// `scripts/web-scope-latency.mjs` times `popErrorScope`, and set this
/// to a few times the slowest frame a working page needs. Visible time
/// only, like `DEADLINE` (`visible_deadline`).
#[cfg(any(target_family = "wasm", test))]
pub(crate) const WORK_DONE_DEADLINE: Duration = Duration::from_secs(30);

/// Whether a frame refuses a scope unresolved at its deadline. Natively
/// yes: nothing else watches the clock. Not on the web, where the
/// watch's page timer does and counts visible time only
/// (`visible_deadline`): the first frame after a hidden tab returns
/// would otherwise refuse a scope whose deadline passed while hidden.
const FRAMES_JUDGE_LATENESS: bool = cfg!(not(target_family = "wasm"));

/// What a scope is begun with: the loop's waker its error wakes, and
/// the time now, from which its deadline runs.
#[derive(Clone, Copy)]
pub(super) struct Turn<'a> {
    pub(super) waker: &'a LoopWaker,
    pub(super) now: Instant,
}

/// The error scopes of one surface not yet seen resolved, each with its
/// deadline, and the first error one caught: once caught, every frame
/// of the surface is refused until it is remounted.
#[derive(Default)]
pub(super) struct Scopes {
    pending: Vec<(Watched, Instant)>,
    caught: Option<String>,
}

impl Scopes {
    /// Runs `call` inside a validation error scope on `device`: refused
    /// when it fails, or when it (or an earlier call) made a GPU error.
    pub(super) fn run<T>(
        &mut self,
        device: &wgpu::Device,
        turn: Turn<'_>,
        call: impl FnOnce() -> Result<T, SurfaceError>,
    ) -> Result<T, Refusal> {
        self.run_submitting(device, None, turn, call)
    }

    /// `run` for a call that submits to `queue`: on the web its deadline
    /// starts when the GPU reports that work done.
    pub(super) fn run_submitting<T>(
        &mut self,
        device: &wgpu::Device,
        queue: Option<&wgpu::Queue>,
        turn: Turn<'_>,
        call: impl FnOnce() -> Result<T, SurfaceError>,
    ) -> Result<T, Refusal> {
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let result = call();
        let work = queue.and_then(work_done);
        self.wait_for_work(Box::pin(scope.pop()), work, turn);
        self.take_resolved();
        let value = result.map_err(Refusal::Renderer)?;
        self.refusal().map(|()| value)
    }

    /// Keeps `scope` until it resolves, refusing at `DEADLINE` from now.
    #[cfg(test)]
    pub(super) fn wait_for(&mut self, scope: Caught, turn: Turn<'_>) {
        self.wait_for_work(scope, None, turn);
    }

    /// `wait_for`, its deadline starting when `work` is reported done.
    fn wait_for_work(&mut self, scope: Caught, work: Option<WorkDone>, turn: Turn<'_>) {
        let watched = Watched::new(scope, work, turn.waker);
        self.pending.push((watched, turn.now + DEADLINE));
    }

    /// The earliest deadline of an unresolved scope, if one is waiting,
    /// for the surface's diagnostic state (`SurfaceState::waiting`).
    pub(super) fn waiting(&self) -> Option<Instant> {
        self.pending.iter().map(|(_, deadline)| *deadline).min()
    }

    /// Whether a scope has caught an error, or timed out, that no frame
    /// has seen: the loop owes a frame that refuses the surface.
    pub(super) fn error_waiting(&mut self) -> bool {
        let mut pending = self.pending.iter_mut();
        pending.any(|(watched, _)| matches!(watched.outcome(), Outcome::Caught(_)))
    }

    /// On each frame of the surface: refused when a scope has caught an
    /// error (now or before), or is unresolved at its deadline.
    pub(super) fn check(&mut self, now: Instant) -> Result<(), Refusal> {
        self.settle(now);
        self.refusal()
    }

    /// Takes in what has resolved, then gives up on what is still
    /// unresolved at its deadline: a late result that did arrive is
    /// judged on what it caught, not refused as a timeout. On the web
    /// the watch's timer judges lateness (`FRAMES_JUDGE_LATENESS`).
    pub(super) fn settle(&mut self, now: Instant) {
        self.take_resolved();
        let late = self.pending.iter().any(|(_, at)| now >= *at);
        if FRAMES_JUDGE_LATENESS && late {
            self.pending.clear();
            self.caught.get_or_insert_with(timed_out);
        }
    }

    /// Drops each resolved scope, keeping the first error caught.
    fn take_resolved(&mut self) {
        let caught = &mut self.caught;
        self.pending
            .retain_mut(|(watched, _)| match watched.outcome() {
                Outcome::Waiting => true,
                Outcome::Clean => false,
                Outcome::Caught(error) => {
                    caught.get_or_insert(error);
                    false
                }
            });
    }

    fn refusal(&self) -> Result<(), Refusal> {
        match &self.caught {
            None => Ok(()),
            Some(error) => Err(Refusal::Renderer(
                format!("{error} (refused until it is remounted)").into(),
            )),
        }
    }
}

/// The refusal of a scope unresolved at its deadline, whether a frame
/// or the web's timer finds it first.
fn timed_out() -> String {
    format!(
        "the GPU did not report this surface's errors within {} ms",
        DEADLINE.as_millis()
    )
}

/// The refusal of work the GPU did not report done by
/// `WORK_DONE_DEADLINE`; the surface's name is added where it is refused.
#[cfg(any(target_family = "wasm", test))]
fn work_timed_out() -> String {
    format!(
        "the GPU did not report this surface's submitted work done within {} ms, so its \
         error scope's own {} ms wait never began (a slow GPU or a hung one: measure with \
         scripts/web-scope-latency.mjs, or raise WORK_DONE_DEADLINE in \
         crates/app/src/painter_surface_scope.rs)",
        WORK_DONE_DEADLINE.as_millis(),
        DEADLINE.as_millis()
    )
}

/// The GPU's report that `queue`'s submitted work is done, on the web;
/// natively a scope resolves when popped, so none.
#[cfg(target_family = "wasm")]
#[allow(clippy::unnecessary_wraps)] // the same signature as the native one, which has none
fn work_done(queue: &wgpu::Queue) -> Option<WorkDone> {
    Some(deadline::submitted_work_done(queue))
}

#[cfg(not(target_family = "wasm"))]
fn work_done(_queue: &wgpu::Queue) -> Option<WorkDone> {
    None
}

#[cfg(test)]
#[path = "painter_surface_scope_fakes.rs"]
pub(super) mod fakes;

#[cfg(test)]
#[path = "painter_surface_scope_tests.rs"]
mod tests;
