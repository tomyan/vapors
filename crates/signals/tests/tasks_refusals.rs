//! Refusals of the task primitive, each loud and naming its fix.

use vapors_signals::{Deadline, Effect, batch, run_ready, set_wake_hook, spawn};

fn hook() {
    set_wake_hook(|| {});
}

#[test]
#[should_panic(expected = "set_wake_hook")]
fn a_wake_with_no_hook_refuses() {
    // Given
    let slot = std::rc::Rc::new(std::cell::RefCell::new(None));
    let kept = std::rc::Rc::clone(&slot);
    spawn(
        Deadline::User,
        std::future::poll_fn(move |cx| {
            *kept.borrow_mut() = Some(cx.waker().clone());
            std::task::Poll::<Result<(), String>>::Pending
        }),
    );
    let _ = run_ready();
    // When
    slot.borrow_mut().take().unwrap().wake();
}

#[test]
#[should_panic(expected = "set_wake_hook called twice")]
fn a_second_hook_refuses() {
    hook();
    hook();
}

#[test]
#[should_panic(expected = "run_ready called inside a batch")]
fn run_ready_inside_a_batch_refuses() {
    let _ = batch(run_ready);
}

#[test]
#[should_panic(expected = "run_ready called inside a batch")]
fn run_ready_inside_an_effect_refuses() {
    Effect::new(|| {
        let _ = run_ready();
    });
}
