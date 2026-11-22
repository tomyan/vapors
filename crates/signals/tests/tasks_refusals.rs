//! Refusals of the task primitive, each loud and naming its fix.

mod common;

use common::hook;
use vapors_signals::{Deadline, Owner, spawn};

#[test]
#[should_panic(expected = "with no wake hook installed")]
fn a_spawn_with_no_hook_refuses() {
    let _ = spawn(Deadline::User, async { Ok::<(), String>(()) });
}

#[test]
#[should_panic(expected = "set_wake_hook called twice")]
fn a_second_hook_refuses() {
    hook();
    hook();
}

#[test]
#[should_panic(expected = "already disposed")]
fn a_spawn_under_a_disposed_owner_refuses() {
    hook();
    let owner = Owner::new();
    owner.dispose();
    owner.with(|| spawn(Deadline::User, async { Ok::<(), String>(()) }));
}
