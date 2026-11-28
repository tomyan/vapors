//! `--check`'s script, a task on the application thread: Live off, on
//! (with a stall of the thread inside), off again, then the end, which
//! wakes the window to judge. Timed by Tokio's timer, not winit's. It
//! switches Live the way Space does: it asks the window, and the window
//! sends `Toggle` to the application thread's handler.

use std::time::Duration;

use vapors_platform::Waker;

use crate::record::{Phase, Shared, lock};

/// How long each off phase lasts: ten ticks.
const PHASE: Duration = Duration::from_millis(1000);

/// How long the stall blocks the thread: three and a half ticks, so a
/// burst of missed ticks would show.
pub const STALL: Duration = Duration::from_millis(350);

pub async fn run(record: Shared, waker: Waker) -> Result<(), String> {
    let mark = |phase| {
        lock(&record)
            .phases
            .push((phase, std::time::Instant::now()));
    };
    let ask = || {
        lock(&record).requests += 1;
        waker.wake().map_err(|closed| closed.to_string())
    };
    mark(Phase::Off);
    tokio::time::sleep(PHASE).await;
    mark(Phase::On);
    ask()?;
    tokio::time::sleep(PHASE / 2).await;
    mark(Phase::Stall);
    std::thread::sleep(STALL);
    tokio::time::sleep(PHASE / 2).await;
    mark(Phase::OffAgain);
    ask()?;
    tokio::time::sleep(PHASE).await;
    mark(Phase::End);
    waker.wake().map_err(|closed| closed.to_string())
}
