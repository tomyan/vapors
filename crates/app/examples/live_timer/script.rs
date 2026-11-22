//! `--check`'s script, a task on the application thread: Live off, on
//! (with a stall of the thread inside), off again, then the end, which
//! wakes the window to judge. Timed by Tokio's timer, not winit's.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use crate::app_side::{Live, mark, toggle};
use crate::record::Phase;

/// How long each phase lasts: ten ticks.
const PHASE: Duration = Duration::from_millis(1000);

/// How long the stall blocks the thread: three and a half ticks, so a
/// burst of missed ticks would show.
const STALL: Duration = Duration::from_millis(350);

pub async fn run(live: Rc<RefCell<Live>>) -> Result<(), String> {
    mark(&live, Phase::Off);
    tokio::time::sleep(PHASE).await;
    mark(&live, Phase::On);
    toggle(&live);
    tokio::time::sleep(PHASE / 2).await;
    mark(&live, Phase::Stall);
    std::thread::sleep(STALL);
    tokio::time::sleep(PHASE / 2).await;
    mark(&live, Phase::OffAgain);
    toggle(&live);
    tokio::time::sleep(PHASE).await;
    mark(&live, Phase::End);
    live.borrow()
        .waker
        .wake()
        .map_err(|closed| closed.to_string())
}
