//! Refusals of the task primitive, each loud and naming its fix.

use vapors_signals::{Deadline, Effect, Signal, batch, run_ready, set_wake_hook, spawn};

fn hook() {
    set_wake_hook(|| {});
}

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
#[should_panic(expected = "run_ready called inside a batch:")]
fn run_ready_inside_a_batch_refuses() {
    let _ = batch(run_ready);
}

#[test]
#[should_panic(expected = "run_ready called inside an effect flush:")]
fn run_ready_inside_an_effect_flush_refuses() {
    // Given: an effect whose first run (inside a batch) is harmless
    let trigger = Signal::new(0);
    Effect::new(move || {
        if trigger.get() > 0 {
            let _ = run_ready();
        }
    });
    // When: a plain set re-runs it from a flush, outside any batch
    trigger.set(1);
}
