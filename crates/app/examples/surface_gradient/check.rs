//! `--check`: the app wrapped to count its frames and to do what the
//! script (`script.rs`, on its own thread) asks of the surface's handle.
//! The handle is not `Send`, so the script sets what it wants in
//! [`Counts`] and wakes the loop; a wake reaches the app only as a
//! frame, and the app acts on the ask in `drawn` after that frame.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use vapors_app::{ElementsApp, Invalidated, SurfaceHandle};
use vapors_platform::{App, Event, Frame, Next, Platform, Waker};

use super::script::script;

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
        })
    }

    /// The script's verdict; refused when it never finished.
    pub fn outcome(&self) -> Result<String, String> {
        let outcome = self.outcome.lock().expect("not poisoned").clone();
        outcome.unwrap_or_else(|| Err("the loop ended before the idle count finished".into()))
    }

    /// Frames and renders so far.
    pub fn now(&self) -> (u64, u64) {
        let frames = self.frames.load(Ordering::SeqCst);
        (frames, self.renders.load(Ordering::SeqCst))
    }

    /// Records a refusal unless a verdict is already there.
    fn refuse(&self, why: String) {
        self.outcome
            .lock()
            .expect("not poisoned")
            .get_or_insert(Err(why));
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
        if let Err(why) = ask.map_or(Ok(()), |ask| self.act(ask)) {
            self.counts.refuse(why);
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
    fn event(&mut self, event: Event, platform: &Platform) {
        self.waker.get_or_insert_with(|| platform.waker());
        self.app.event(event, platform);
    }

    fn frame(&mut self, frame: &Frame<'_>) -> Next {
        self.counts.frames.fetch_add(1, Ordering::SeqCst);
        if !self.started {
            self.started = true;
            if let Err(why) = self.start_script() {
                self.counts.refuse(why);
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
        self.act_if_asked();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_refusal_is_kept() {
        // Given
        let counts = Counts::new(Arc::new(AtomicU64::new(0)));

        // When
        counts.refuse("first".into());
        counts.refuse("second".into());

        // Then
        assert_eq!(counts.outcome(), Err("first".into()));
    }
}
