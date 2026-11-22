//! The second process of the lock tests (`gate_hold_process_tests.rs`):
//! this test binary run again on its ignored test `lock_child`, which
//! takes the hold `XTASK_LOCK_CHILD` names in the lock directory
//! `XTASK_LOCK_DIR`, says each step on stdout as `child: <what>`, and
//! keeps what it holds until its stdin closes. The parent's side, which
//! starts it and reads those lines, is `lock_child_handle.rs`.

use std::io::{BufRead, Read, Write};
use std::path::Path;
use std::time::Duration;

use crate::gate_hold::{GateHolds, Hold};
use crate::lock_place::Place;
use crate::quiet_lock::{self, Form, Holder, LONGEST};
use crate::share::{self, Sharer};

/// The hold the child takes.
pub const ACTION: &str = "XTASK_LOCK_CHILD";
/// The lock directory it takes it in.
pub const DIR: &str = "XTASK_LOCK_DIR";
/// The worktree it names itself as.
pub const WORKTREE: &str = "XTASK_LOCK_WORKTREE";
/// Each of its waits, in milliseconds.
pub const WAIT_MS: &str = "XTASK_LOCK_WAIT_MS";
#[test]
#[ignore = "the second process of the lock tests, started by them, not a test of its own"]
fn lock_child() {
    let Ok(action) = std::env::var(ACTION) else {
        return;
    };
    let var = |name: &str| std::env::var(name).unwrap_or_else(|_| panic!("{name} is unset"));
    let wait = Duration::from_millis(var(WAIT_MS).parse().expect("a wait in ms"));
    act(
        &action,
        Place::brief(Path::new(&var(DIR)), &var(WORKTREE), wait),
    );
}

/// Take what `action` names at `place`, saying so, and keep it until
/// stdin closes.
fn act(action: &str, place: Place) {
    match action {
        "share" | "exclusive" | "switch" => gate(action, place),
        "old-gate" => old_gate(&place),
        "old-quiet" => old_quiet(&place),
        "nested-test" => nested_test(place),
        other => panic!("{ACTION}={other}, not an action this child takes"),
    }
}

/// A gate's holds: shared, exclusive, or shared then exclusive once a
/// line comes on stdin.
fn gate(action: &str, place: Place) {
    let mut holds = GateHolds::new(place);
    let first = if action == "exclusive" {
        Hold::Exclusive
    } else {
        Hold::Shared
    };
    let first_line = if action == "switch" { "shared" } else { "held" };
    if said(holds.hold(first), first_line).is_none() {
        return;
    }
    if action == "switch" {
        std::io::stdin()
            .lock()
            .read_line(&mut String::new())
            .expect("stdin");
        if said(holds.hold(Hold::Exclusive), "exclusive").is_none() {
            return;
        }
    }
    until_stdin_closes();
}

/// An old-protocol gate, as `gate_floor::begin` took its locks before
/// holds were per step: the quiet lock as a gate, then the gate lock
/// exclusive, for its whole run.
fn old_gate(place: &Place) {
    let holder = Holder::me(&place.worktree, LONGEST, Form::Gate, "gate");
    let held = quiet_lock::take(&place.dir, holder, place.quiet_wait).and_then(|quiet| {
        crate::gate_lock::acquire(&place.dir, &place.worktree, place.lock_wait)
            .map(|lock| (quiet, lock))
    });
    if let Some(_held) = said(held, "held") {
        until_stdin_closes();
    }
}

/// An old-protocol gate between taking the quiet lock and the gate lock.
fn old_quiet(place: &Place) {
    let holder = Holder::me(&place.worktree, LONGEST, Form::Gate, "gate");
    if let Some(_quiet) = said(
        quiet_lock::take(&place.dir, holder, place.quiet_wait),
        "held",
    ) {
        until_stdin_closes();
    }
}

/// What `cargo xtask test` does about the gate lock, with this process's
/// own `XTASK_GATE`.
fn nested_test(place: Place) {
    let gate_env = std::env::var(crate::priority::GATE_ENV).ok();
    match share::unless_in_gate_at(Sharer::Test, gate_env.as_deref(), || Ok(place)) {
        Ok(None) => say("went on"),
        Ok(Some(_)) => say("shared"),
        Err(why) => say(&format!("refused: {why}")),
    }
}

/// Say `line` when `taken` is, else its refusal; what was taken.
fn said<T>(taken: Result<T, String>, line: &str) -> Option<T> {
    match taken {
        Ok(held) => {
            say(line);
            Some(held)
        }
        Err(why) => {
            say(&format!("refused: {why}"));
            None
        }
    }
}

fn say(line: &str) {
    println!("child: {line}");
    std::io::stdout().flush().expect("stdout");
}

fn until_stdin_closes() {
    std::io::stdin()
        .lock()
        .read_to_end(&mut Vec::new())
        .expect("stdin");
}
