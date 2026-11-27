//! The application thread's side: the handler and the Live switch, and
//! with `--check` the script. App-authored code, so all of it runs on the
//! application thread.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use tokio::time::MissedTickBehavior;
use vapors_platform::Waker;
use vapors_signals::{Deadline, Owner, spawn};

use crate::TICK;
use crate::record::{Shared, lock};

/// What the UI thread sends.
pub enum ToApp {
    /// The window is up: the first message.
    Started,
    /// Space was pressed, or the script asked.
    Toggle,
}

/// What the handler reaches.
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
pub fn make(mut live: Live, check: bool) -> impl FnMut(ToApp) {
    if check {
        let (record, waker) = (Arc::clone(&live.record), live.waker.clone());
        // Deadline::User: the script ends itself; the watchdog bounds it
        spawn(Deadline::User, crate::script::run(record, waker));
    }
    move |message| {
        lock(&live.record)
            .handlers
            .push(std::thread::current().id());
        if let ToApp::Toggle = message {
            toggle(&mut live);
        }
    }
}

/// Switches Live: on spawns the interval under a new owner, off
/// disposes that owner, which drops the interval and its timer.
fn toggle(live: &mut Live) {
    let on = live.running.is_none();
    lock(&live.record)
        .toggles
        .push((Instant::now(), std::thread::current().id(), on));
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
    interval.set_missed_tick_behavior(MissedTickBehavior::Burst);
    interval.tick().await;
    loop {
        interval.tick().await;
        lock(&record)
            .ticks
            .push((Instant::now(), std::thread::current().id()));
        shown.fetch_add(1, Ordering::SeqCst);
        waker.wake().map_err(|closed| closed.to_string())?;
    }
}
