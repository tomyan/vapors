//! When the app's `App::start` runs (one-scene F0): once, after the GPU
//! is ready and once the window has a size, before the first frame. A
//! window with no size (minimised at launch, or a web canvas not yet
//! laid out) is waited for until [`SIZE_DEADLINE`], then refused.

use std::time::Duration;

use crate::Instant;

/// How long after the GPU is ready the window may stay without a size.
///
/// Measured 2026-11-13 on the dev machine under Xvfb, three runs each of
/// `surface_gradient --check` and `wake_window`: the start ran 0.1 ms
/// after the GPU was ready every time, the window already sized (the
/// start-up line prints it). The web page's canvas was not measured.
/// 10 s is far beyond that and leaves a person time to restore a window
/// launched minimised; a window kept without a size longer is refused,
/// so a blank window never waits silently.
pub(crate) const SIZE_DEADLINE: Duration = Duration::from_secs(10);

/// What the handler does next about the app's start.
#[derive(Debug, PartialEq)]
pub(crate) enum Step {
    /// Call `App::start` now.
    Start,
    /// Nothing to do yet: no GPU.
    Wait,
    /// Wake the loop at this time, if no resize comes first.
    WaitUntil(Instant),
    /// Stop the loop with this message.
    Refuse(String),
    /// The start has run.
    Started,
}

/// Where the app's start is.
#[derive(Default)]
pub(crate) struct StartGate {
    ready_at: Option<Instant>,
    started: bool,
}

impl StartGate {
    /// The GPU became ready at `now`.
    pub(crate) fn gpu_ready(&mut self, now: Instant) {
        self.ready_at.get_or_insert(now);
    }

    /// How long since the GPU was ready, at `now`.
    pub(crate) fn since_ready(&self, now: Instant) -> Duration {
        self.ready_at.map_or(Duration::ZERO, |ready| now - ready)
    }

    /// Whether the app has started.
    pub(crate) fn started(&self) -> bool {
        self.started
    }

    /// What to do with the window at `size` (width, height) at `now`.
    pub(crate) fn step(&mut self, (width, height): (u32, u32), now: Instant) -> Step {
        if self.started {
            return Step::Started;
        }
        let Some(ready_at) = self.ready_at else {
            return Step::Wait;
        };
        if width > 0 && height > 0 {
            self.started = true;
            return Step::Start;
        }
        let deadline = ready_at + SIZE_DEADLINE;
        if now >= deadline {
            return Step::Refuse(no_size(width, height));
        }
        Step::WaitUntil(deadline)
    }
}

fn no_size(width: u32, height: u32) -> String {
    format!(
        "the window is still {width}x{height} {} s after the GPU was ready, so the app cannot \
         lay its first frame out; fix: show the window (on the web, give the canvas a size) \
         within {} s of launch",
        SIZE_DEADLINE.as_secs(),
        SIZE_DEADLINE.as_secs()
    )
}

#[cfg(test)]
#[path = "start_gate_tests.rs"]
mod tests;
