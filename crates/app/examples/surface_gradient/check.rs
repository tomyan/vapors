//! `--check`: the app wrapped to count its frames and to do what the
//! script (`script.rs`, on its own thread) asks of the surface's handle.
//! The handle is not `Send`, so the script sets what it wants in
//! [`Counts`] and wakes the loop; a wake reaches the app only as a
//! frame, and the app acts on the ask in `drawn` after that frame.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use vapors_app::{ElementsApp, Invalidated, SurfaceHandle};
use vapors_platform::{App, Event, Frame, Next, Platform, Start, Waker};

use super::script::script;
use super::trace::Trace;

/// What the script asks the app to do with the surface's handle.
#[derive(Clone, Copy, Debug)]
pub enum Ask {
    Invalidate,
    RequestAnimation,
    StopAnimation,
}

/// What the app and the script share.
pub struct Counts {
    frames: AtomicU64,
    renders: Arc<AtomicU64>,
    exit: AtomicBool,
    /// Set by the script; the app takes it in `drawn`.
    ask: Mutex<Option<Ask>>,
    outcome: Mutex<Option<Result<String, String>>>,
    /// What the app did each frame; printed with a failure.
    trace: Mutex<Trace>,
}

impl Counts {
    /// Counts with `renders` counted by the renderer.
    pub fn new(renders: Arc<AtomicU64>) -> Arc<Self> {
        Arc::new(Self {
            frames: AtomicU64::new(0),
            renders,
            exit: AtomicBool::new(false),
            ask: Mutex::new(None),
            outcome: Mutex::new(None),
            trace: Mutex::new(Trace::default()),
        })
    }

    /// The script's verdict; refused when it never finished.
    pub fn outcome(&self) -> Result<String, String> {
        let outcome = self.outcome.lock().expect("not poisoned").clone();
        outcome.unwrap_or_else(|| Err("the loop ended before the idle count finished".into()))
    }

    /// Frames and renders so far, between frames: a frame holds the
    /// trace lock from before its render until it is counted, so a count
    /// never lands inside one.
    pub fn now(&self) -> (u64, u64) {
        let _between_frames = self.trace.lock().expect("not poisoned");
        let frames = self.frames.load(Ordering::SeqCst);
        (frames, self.renders.load(Ordering::SeqCst))
    }

    /// Records a refusal unless a verdict is already there.
    fn refuse(&self, why: &str) {
        let why = self.with_trace(why);
        self.outcome
            .lock()
            .expect("not poisoned")
            .get_or_insert(Err(why));
    }

    /// `why` followed by the per-frame trace.
    fn with_trace(&self, why: &str) -> String {
        let trace = self.trace.lock().expect("not poisoned").report();
        format!(
            "{why}\nper frame, from the first (a run of identical frames is one line):\n{trace}"
        )
    }

    /// Leaves `ask` for the app and wakes the loop; refused when the
    /// loop has ended.
    pub fn ask(&self, ask: Ask, waker: &Waker) -> Result<(), String> {
        *self.ask.lock().expect("not poisoned") = Some(ask);
        waker
            .wake()
            .map_err(|_| format!("the loop ended before the script asked to {ask:?}"))
    }

    /// Keeps the first verdict: a refusal from the app stands.
    pub fn finish(&self, outcome: Result<String, String>, waker: &Waker) {
        let outcome = outcome.map_err(|why| self.with_trace(&why));
        let _ = self
            .outcome
            .lock()
            .expect("not poisoned")
            .get_or_insert(outcome);
        self.exit.store(true, Ordering::SeqCst);
        if waker.wake().is_err() {
            eprintln!("surface_gradient: the loop ended before the script's last wake");
        }
    }
}

/// The app, counting frames and starting the script on the first.
pub struct Checked {
    app: ElementsApp,
    handle: SurfaceHandle,
    counts: Arc<Counts>,
    waker: Option<Waker>,
    started: bool,
}

impl Checked {
    pub fn new(app: ElementsApp, handle: SurfaceHandle, counts: Arc<Counts>) -> Self {
        Self {
            app,
            handle,
            counts,
            waker: None,
            started: false,
        }
    }

    fn start_script(&mut self) -> Result<(), String> {
        let waker = self
            .waker
            .clone()
            .ok_or("no event came before the first frame, so there was no waker")?;
        let counts = Arc::clone(&self.counts);
        std::thread::spawn(move || script(&counts, &waker));
        Ok(())
    }

    /// Does what the script asked, if anything.
    fn act_if_asked(&self) {
        let ask = self.counts.ask.lock().expect("not poisoned").take();
        if let Some(ask) = ask {
            let number = self.counts.frames.load(Ordering::SeqCst);
            let mut trace = self.counts.trace.lock().expect("not poisoned");
            trace.note(number, format!("the app acted on {ask:?}"));
        }
        if let Err(why) = ask.map_or(Ok(()), |ask| self.act(ask)) {
            self.counts.refuse(&why);
        }
    }

    /// Refused unless an invalidate or request woke the loop, or a stop
    /// was done.
    fn act(&self, ask: Ask) -> Result<(), String> {
        let answer = match ask {
            Ask::Invalidate => self.handle.invalidate(),
            Ask::RequestAnimation => self.handle.request_animation(),
            Ask::StopAnimation => {
                return self.handle.stop_animation().map_err(|error| {
                    format!("the running app's stop_animation was refused: {error}")
                });
            }
        };
        match answer {
            Ok(Invalidated::Woke) => Ok(()),
            other => Err(format!(
                "the running app's {ask:?} gave {other:?}, not Ok(Woke)"
            )),
        }
    }
}

impl App for Checked {
    fn start(&mut self, start: &Start<'_>) -> Result<(), String> {
        self.app.start(start)
    }

    fn event(&mut self, event: Event, platform: &Platform) {
        self.waker.get_or_insert_with(|| platform.waker());
        self.app.event(event, platform);
    }

    fn frame(&mut self, frame: &Frame<'_>) -> Next {
        if !self.started {
            self.started = true;
            if let Err(why) = self.start_script() {
                self.counts.refuse(&why);
                return Next::Exit;
            }
        }
        if self.counts.exit.load(Ordering::SeqCst) {
            return Next::Exit;
        }
        let mut trace = self.counts.trace.lock().expect("not poisoned");
        let renders_before = self.counts.renders.load(Ordering::SeqCst);
        let next = self.app.frame(frame);
        let rendered = self.counts.renders.load(Ordering::SeqCst) > renders_before;
        let number = self.counts.frames.fetch_add(1, Ordering::SeqCst) + 1;
        let (decision, states) = (self.app.last_decision(), self.app.surface_states());
        trace.frame(number, next, decision, &states, rendered);
        next
    }

    fn drawn(&mut self, platform: &Platform) {
        self.app.drawn(platform);
        self.act_if_asked();
    }
}

#[cfg(test)]
#[path = "check_tests.rs"]
mod tests;
