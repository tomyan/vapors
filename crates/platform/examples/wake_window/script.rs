//! `wake_window`'s script, on its own thread: settle, count idle frames,
//! wake singly, wake in a burst (`burst.rs`), wait until a time
//! (`timed.rs`), then end the loop with a last wake.

use std::time::Duration;

use vapors_platform::Waker;

use crate::burst::{self, Burst};
use crate::burst_judge::Outcome;
use crate::shared::{Shared, frames_over};
use crate::timed::{self, Timed};

/// After the first frame, before the idle count. The window's own
/// start-up frames (mapped, sized, exposed) land in this; 2 s is far
/// past the last seen, which `run` prints, so a late one cannot make
/// the idle count flaky. Frames from the window system are not told
/// apart from wakes: the app never sees an expose.
const SETTLE: Duration = Duration::from_secs(2);
/// How long the app is left alone, expecting no frame.
const IDLE: Duration = Duration::from_secs(1);
/// Between wakes: llvmpipe draws this window in well under 50 ms.
const BETWEEN: Duration = Duration::from_millis(300);
const SINGLE_WAKES: usize = 3;

/// The whole script, then a wake that ends the loop.
pub fn run(waker: &Waker, shared: &Shared) {
    let outcome = measure(waker, shared);
    shared.finish(outcome);
    if let Err(closed) = waker.wake() {
        eprintln!("wake_window: the last wake failed ({closed}) though the loop should be running");
        std::process::exit(1);
    }
}

fn measure(waker: &Waker, shared: &Shared) -> Result<String, String> {
    std::thread::sleep(SETTLE);
    let startup = (shared.count(), shared.startup_span());
    let idle = frames_over(shared, || {
        std::thread::sleep(IDLE);
        Ok(())
    })?;
    let single = (0..SINGLE_WAKES)
        .map(|_| frames_over(shared, || wake_then_wait(waker)))
        .collect::<Result<Vec<_>, _>>()?;
    let burst = burst::measure(waker, shared)?;
    let timed = timed::measure(waker, shared)?;
    let line = format!(
        "start-up {} frames over {:?}; idle frames in {IDLE:?}: {idle}; per wake: {single:?}; {burst} {timed}",
        startup.0, startup.1
    );
    let failures = failures(idle, &single, &burst, &timed);
    if failures.is_empty() {
        Ok(line)
    } else {
        Err(format!("{line}\n{}", failures.join("\n")))
    }
}

/// Every count that did not hold, and the burst's refusal if it had one.
fn failures(idle: u64, single: &[u64], burst: &Burst, timed: &Timed) -> Vec<String> {
    let mut failures = Vec::new();
    if idle != 0 {
        failures.push(format!("expected 0 idle frames, got {idle}"));
    }
    if single.iter().any(|n| *n != 1) {
        failures.push(format!("expected 1 frame per wake, got {single:?}"));
    }
    if let Outcome::Count(why) | Outcome::Refused(why) = &burst.outcome {
        failures.push(why.clone());
    }
    if !timed.held() {
        failures.push(format!("expected {}", timed::EXPECTED));
    }
    failures
}

fn wake_then_wait(waker: &Waker) -> Result<(), String> {
    waker.wake().map_err(|closed| closed.to_string())?;
    std::thread::sleep(BETWEEN);
    Ok(())
}
