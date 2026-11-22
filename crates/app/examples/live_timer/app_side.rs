//! The application thread's side: the handlers, the Live switch and,
//! with `--check`, the script that drives it. App-authored code, so all
//! of it runs on the application thread.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use tokio::time::MissedTickBehavior;
use vapors_platform::Waker;
use vapors_signals::{Deadline, Owner, spawn};

use crate::TICK;
use crate::record::{Phase, Shared, lock};

/// What the UI thread sends.
pub enum ToApp {
    /// The window is up: the first message, from the UI thread.
    Started,
    /// Space was pressed.
    Toggle,
}

/// What both the handler and the script reach.
pub struct Live {
    pub record: Shared,
    /// The tick the window shows, read by the UI thread.
    pub shown: Arc<AtomicU64>,
    pub waker: Waker,
    /// The owner of the running interval's task while Live is on.
    pub running: Option<Owner>,
}

/// Builds the app side on the application thread: its handler, and with
/// `check` the script.
pub fn make(live: Live, check: bool) -> impl FnMut(ToApp) {
    let live = Rc::new(RefCell::new(live));
    if check {
        let script = Rc::clone(&live);
        // Deadline::User: the script ends itself; the watchdog bounds it
        spawn(Deadline::User, crate::script::run(script));
    }
    move |message| {
        let record = Rc::clone(&live);
        lock(&record.borrow().record)
            .handlers
            .push(std::thread::current().id());
        if let ToApp::Toggle = message {
            toggle(&live);
        }
    }
}

/// Switches Live: on spawns the interval under a new owner, off
/// disposes that owner, which drops the interval and its timer.
pub fn toggle(live: &Rc<RefCell<Live>>) {
    let mut live = live.borrow_mut();
    lock(&live.record)
        .handlers
        .push(std::thread::current().id());
    if let Some(owner) = live.running.take() {
        owner.dispose();
        return;
    }
    let owner = Owner::new();
    let ticking = (
        Arc::clone(&live.record),
        Arc::clone(&live.shown),
        live.waker.clone(),
    );
    // Deadline::User: Live runs until the user switches it off
    owner.with(|| spawn(Deadline::User, ticks(ticking)));
    live.running = Some(owner);
}

/// One tick every `TICK`, the first a full period after the switch; after
/// a stall, one tick and then a full period (`MissedTickBehavior::Delay`).
/// Each tick is shown by a frame wake.
async fn ticks((record, shown, waker): (Shared, Arc<AtomicU64>, Waker)) -> Result<(), String> {
    let mut interval = tokio::time::interval(TICK);
    interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
    interval.tick().await;
    loop {
        interval.tick().await;
        shown.fetch_add(1, Ordering::SeqCst);
        lock(&record)
            .ticks
            .push((Instant::now(), std::thread::current().id()));
        waker.wake().map_err(|closed| closed.to_string())?;
    }
}

/// Marks the start of `phase`.
pub fn mark(live: &Rc<RefCell<Live>>, phase: Phase) {
    lock(&live.borrow().record)
        .phases
        .push((phase, Instant::now()));
}
