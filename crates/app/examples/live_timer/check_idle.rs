//! Live off: no frames, no ticks, and no turns of the application thread
//! beyond the ones the script itself takes.

use crate::check::Window;
use crate::record::{Drawn, Record};

/// Turns before the window serves the request to switch Live on: the
/// `Started` message, the script's first poll, and its 1 s timer.
const OFF_TURNS: u64 = 3;

/// Turns from the request to switch Live off to the end: the `Toggle`
/// message, at most one tick already on its way, and the script's last
/// timer.
const OFF_AGAIN_TURNS: u64 = 3;

pub fn problems(record: &Record, window: &Window) -> Vec<String> {
    let mut found = Vec::new();
    let own = |frame: &&Drawn| !frame.request;
    let before = record
        .frames
        .iter()
        .filter(own)
        .filter(|f| f.at > window.off && f.at < window.on_at)
        .count();
    let after = record
        .frames
        .iter()
        .filter(own)
        .filter(|f| f.at > window.off_at)
        .count();
    let ticks_after = record
        .ticks
        .iter()
        .filter(|(at, _)| *at > window.off_at)
        .count();
    if before > 0 || after > 1 || ticks_after > 0 {
        found.push(format!(
            "with Live off: {before} frames before it went on; {after} frames (at most one, \
             for a tick on its way) and {ticks_after} ticks after it went off; fix: the \
             interval's owner, or what else wakes the window"
        ));
    }
    found.extend(turns(record));
    found
}

/// The turns the window read at the two request frames and the end.
fn turns(record: &Record) -> Option<String> {
    let [on, off, end] = record.turns.as_slice() else {
        return Some(format!(
            "{} turn readings, three expected; fix: the window's request frames",
            record.turns.len()
        ));
    };
    let again = end.saturating_sub(*off);
    (*on > OFF_TURNS || again > OFF_AGAIN_TURNS).then(|| {
        format!(
            "the application thread took {on} turns with Live off at the start (at most \
             {OFF_TURNS}) and {again} after it went off (at most {OFF_AGAIN_TURNS}); fix: \
             what wakes the idle thread"
        )
    })
}
