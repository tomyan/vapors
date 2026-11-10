//! `--check`: the app wrapped to count its frames, and a script on its
//! own thread that settles, counts frames and renders over an idle
//! second, asks the app to invalidate the surface, counts again, then
//! ends the loop with a wake. A wake reaches the app only as a frame,
//! so the app invalidates in `drawn` after the script's wake frame: one
//! frame for that wake, one for the invalidate's own wake, one render.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use vapors_app::{ElementsApp, Invalidated, SurfaceHandle};
use vapors_platform::{App, Event, Frame, Next, Platform, Waker};

/// After the first frame, before the idle count: the window's own
/// start-up frames land in this. `wake_window` measured them well
/// inside 2 s under Xvfb and lavapipe.
const SETTLE: Duration = Duration::from_secs(2);
/// How long the app is left alone, expecting no frame and no render.
const IDLE: Duration = Duration::from_secs(1);

/// What the app and the script share.
pub struct Counts {
    frames: AtomicU64,
    renders: Arc<AtomicU64>,
    exit: AtomicBool,
    /// Set by the script; the app invalidates the surface on seeing it.
    invalidate: AtomicBool,
    outcome: Mutex<Option<Result<String, String>>>,
}

impl Counts {
    /// Counts with `renders` counted by the renderer.
    pub fn new(renders: Arc<AtomicU64>) -> Arc<Self> {
        Arc::new(Self {
            frames: AtomicU64::new(0),
            renders,
            exit: AtomicBool::new(false),
            invalidate: AtomicBool::new(false),
            outcome: Mutex::new(None),
        })
    }

    /// The script's verdict; refused when it never finished.
    pub fn outcome(&self) -> Result<String, String> {
        let outcome = self.outcome.lock().expect("not poisoned").clone();
        outcome.unwrap_or_else(|| Err("the loop ended before the idle count finished".into()))
    }

    fn now(&self) -> (u64, u64) {
        let frames = self.frames.load(Ordering::SeqCst);
        (frames, self.renders.load(Ordering::SeqCst))
    }

    /// Keeps the first verdict: a refusal from the app stands.
    fn finish(&self, outcome: Result<String, String>, waker: &Waker) {
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

    /// Invalidates the surface when the script asked; refused unless
    /// the invalidate woke the loop.
    fn invalidate_if_asked(&self) {
        if !self.counts.invalidate.swap(false, Ordering::SeqCst) {
            return;
        }
        let invalidated = self.handle.invalidate();
        if invalidated != Ok(Invalidated::Woke) {
            let why = format!("the running app's invalidate gave {invalidated:?}, not Ok(Woke)");
            *self.counts.outcome.lock().expect("not poisoned") = Some(Err(why));
        }
    }
}

impl App for Checked {
    fn event(&mut self, event: Event, platform: &Platform) {
        self.waker.get_or_insert_with(|| platform.waker());
        self.app.event(event, platform);
    }

    fn frame(&mut self, frame: &Frame<'_>) -> Next {
        self.counts.frames.fetch_add(1, Ordering::SeqCst);
        if !self.started {
            self.started = true;
            if let Err(why) = self.start_script() {
                *self.counts.outcome.lock().expect("not poisoned") = Some(Err(why));
                return Next::Exit;
            }
        }
        if self.counts.exit.load(Ordering::SeqCst) {
            return Next::Exit;
        }
        self.app.frame(frame)
    }

    fn drawn(&mut self, platform: &Platform) {
        self.app.drawn(platform);
        self.invalidate_if_asked();
    }
}

/// Settle, count an idle second, have the app invalidate, end the loop.
fn script(counts: &Counts, waker: &Waker) {
    std::thread::sleep(SETTLE);
    let settled = counts.now();
    std::thread::sleep(IDLE);
    let idle = counts.now();
    counts.invalidate.store(true, Ordering::SeqCst);
    if waker.wake().is_err() {
        return counts.finish(Err("the loop ended before the invalidate".into()), waker);
    }
    std::thread::sleep(IDLE);
    let invalidated = since(idle, counts.now());
    counts.finish(verdict(settled.1, since(settled, idle), invalidated), waker);
}

/// Frames and renders from `then` to `now`.
fn since(then: (u64, u64), now: (u64, u64)) -> (u64, u64) {
    (now.0 - then.0, now.1 - then.1)
}

/// Passes when the gradient rendered while settling, nothing was drawn
/// or rendered while idle, and the invalidate drew two frames (the
/// script's wake, the invalidate's wake) and rendered once.
fn verdict(
    settled_renders: u64,
    (frames, renders): (u64, u64),
    invalidated: (u64, u64),
) -> Result<String, String> {
    if settled_renders == 0 {
        return Err("the gradient never rendered".to_owned());
    }
    if frames + renders > 0 {
        return Err(format!(
            "idle for {IDLE:?}, the app drew {frames} frames and rendered {renders} times; expected none"
        ));
    }
    if invalidated != (2, 1) {
        return Err(format!(
            "after one invalidate the app drew {} frames and rendered {} times; expected 2 frames (the script's wake, the invalidate's) and 1 render",
            invalidated.0, invalidated.1
        ));
    }
    Ok(format!(
        "{settled_renders} render(s) settling, 0 frames and 0 renders idle for {IDLE:?}, 2 frames and 1 render after an invalidate"
    ))
}
