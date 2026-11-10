//! `--check`'s script, on its own thread: settles, counts frames and
//! renders over an idle second, has the app invalidate the surface and
//! counts again, has it request animation and counts a second of
//! frames, has it stop and counts an idle second, then ends the loop.

use std::time::Duration;

use vapors_platform::Waker;

use super::check::{Ask, Counts};

/// After the first frame, before the idle count: the window's own
/// start-up frames land in this. `wake_window` measured them well
/// inside 2 s under Xvfb and lavapipe.
const SETTLE: Duration = Duration::from_secs(2);
/// How long each count lasts.
const IDLE: Duration = Duration::from_secs(1);
/// After a stop, for the frame in flight to end before the idle count:
/// at `MIN_ANIMATING` frames a second a frame takes 100 ms.
const STOPPING: Duration = Duration::from_millis(250);
/// The fewest frames an animating second may draw. Under Xvfb and
/// llvmpipe on the dev machine (2026-11-10, 30 runs) it drew 227 to 244;
/// 10 leaves room for a slower CI runner and is far above the 0 of a
/// loop that waits.
const MIN_ANIMATING: u64 = 10;

/// Frames and renders over each count.
struct Measured {
    settled_renders: u64,
    idle: (u64, u64),
    invalidated: (u64, u64),
    animated: (u64, u64),
    stopped: (u64, u64),
}

/// Runs the counts and leaves the verdict in `counts`.
pub fn script(counts: &Counts, waker: &Waker) {
    let outcome = measure(counts, waker).and_then(|measured| verdict(&measured));
    counts.finish(outcome, waker);
}

fn measure(counts: &Counts, waker: &Waker) -> Result<Measured, String> {
    std::thread::sleep(SETTLE);
    let settled = counts.now();
    let idle = count(counts, None, waker)?;
    let invalidated = count(counts, Some(Ask::Invalidate), waker)?;
    let animated = count(counts, Some(Ask::RequestAnimation), waker)?;
    counts.ask(Ask::StopAnimation, waker)?;
    std::thread::sleep(STOPPING);
    let stopped = count(counts, None, waker)?;
    Ok(Measured {
        settled_renders: settled.1,
        idle,
        invalidated,
        animated,
        stopped,
    })
}

/// Frames and renders over `IDLE`, after asking for `ask`.
fn count(counts: &Counts, ask: Option<Ask>, waker: &Waker) -> Result<(u64, u64), String> {
    let then = counts.now();
    if let Some(ask) = ask {
        counts.ask(ask, waker)?;
    }
    std::thread::sleep(IDLE);
    let now = counts.now();
    Ok((now.0 - then.0, now.1 - then.1))
}

/// Passes when the gradient rendered while settling, nothing was drawn
/// or rendered while idle, the invalidate drew two frames (the script's
/// wake, the invalidate's wake) and rendered once, the animating second
/// drew at least `MIN_ANIMATING` frames rendering in each but the
/// script's wake frame, and nothing was drawn after the stop.
fn verdict(measured: &Measured) -> Result<String, String> {
    if measured.settled_renders == 0 {
        return Err("the gradient never rendered".to_owned());
    }
    still("idle", measured.idle)?;
    if measured.invalidated != (2, 1) {
        let (frames, renders) = measured.invalidated;
        return Err(format!(
            "after one invalidate the app drew {frames} frames and rendered {renders} times; expected 2 frames (the script's wake, the invalidate's) and 1 render"
        ));
    }
    animating(measured.animated)?;
    still("after stop_animation", measured.stopped)?;
    Ok(format!(
        "{} render(s) settling, 0 frames and 0 renders idle for {IDLE:?}, 2 frames and 1 render after an invalidate, {} frames and {} renders animating for {IDLE:?}, 0 frames and 0 renders after stop_animation",
        measured.settled_renders, measured.animated.0, measured.animated.1
    ))
}

/// Refused unless the app drew and rendered nothing.
fn still(when: &str, (frames, renders): (u64, u64)) -> Result<(), String> {
    if frames + renders > 0 {
        return Err(format!(
            "{when} for {IDLE:?}, the app drew {frames} frames and rendered {renders} times; expected none"
        ));
    }
    Ok(())
}

/// Refused unless at least `MIN_ANIMATING` frames were drawn and
/// exactly one drew without rendering: the script's own wake frame,
/// which finds nothing to render. `Counts::now` is read between frames,
/// so no frame is cut by the count's start or end.
fn animating((frames, renders): (u64, u64)) -> Result<(), String> {
    if frames < MIN_ANIMATING || renders + 1 != frames {
        return Err(format!(
            "animating for {IDLE:?}, the app drew {frames} frames and rendered {renders} times; expected at least {MIN_ANIMATING} frames, all rendering but exactly one (the script's wake frame)"
        ));
    }
    Ok(())
}

#[cfg(test)]
#[path = "script_tests.rs"]
mod tests;
