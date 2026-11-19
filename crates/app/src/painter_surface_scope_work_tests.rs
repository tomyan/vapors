//! A watched scope's deadline starting when the GPU reports its work
//! done (`staged_deadline`): the page's timers and the GPU's report are
//! driven by the test.

use std::cell::{Cell, RefCell};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use super::hidden_tests::FakePage;
use super::{LoopWaker, Outcome, Watch};
use crate::painter::surfaces::scope::deadline::staged_deadline;
use crate::painter::surfaces::scope::fakes::{Late, counting, woken};
use crate::painter::surfaces::scope::{DEADLINE, WORK_DONE_DEADLINE, timed_out, work_timed_out};

/// The GPU's report of work done: resolves once the test says.
#[derive(Clone, Default)]
struct Reported(Rc<Cell<bool>>);

impl Future for Reported {
    type Output = ();
    fn poll(self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<()> {
        if self.0.get() {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

/// A watch on `late` whose deadline starts at `work`'s report, polled
/// once, as the page's task queue drives one.
fn racing(
    late: &Late,
    work: &Reported,
    page: &FakePage,
    waker: &LoopWaker,
) -> (Watch, Rc<RefCell<Outcome>>) {
    let outcome = Rc::new(RefCell::new(Outcome::Waiting));
    let deadline = Box::pin(staged_deadline(page.clone(), Box::pin(work.clone())));
    let mut watch = Watch::new(Box::pin(late.clone()), deadline, &outcome, waker);
    assert_eq!(poll(&mut watch), Poll::Pending);
    (watch, outcome)
}

fn poll(watch: &mut Watch) -> Poll<()> {
    Pin::new(watch).poll(&mut Context::from_waker(Waker::noop()))
}

#[test]
fn a_slow_work_done_then_a_prompt_clean_scope_is_not_refused() {
    // Given
    let (count, waker) = counting();
    let (late, work, page) = (Late::default(), Reported::default(), FakePage::default());
    let (mut watch, outcome) = racing(&late, &work, &page, &waker);

    // When: the GPU draws for longer than DEADLINE, then reports
    let while_drawing = poll(&mut watch);
    work.0.set(true);
    let reported = poll(&mut watch);
    late.resolve(None);
    let resolved = poll(&mut watch);

    // Then
    assert_eq!(page.armed(), [WORK_DONE_DEADLINE, DEADLINE]);
    assert_eq!((while_drawing, reported), (Poll::Pending, Poll::Pending));
    assert_eq!(resolved, Poll::Ready(()));
    assert_eq!(*outcome.borrow(), Outcome::Clean);
    assert_eq!(woken(&count), 0);
}

#[test]
fn a_prompt_work_done_then_a_scope_unresolved_past_the_deadline_is_refused() {
    // Given
    let (count, waker) = counting();
    let (late, work, page) = (Late::default(), Reported::default(), FakePage::default());
    let (mut watch, outcome) = racing(&late, &work, &page, &waker);
    work.0.set(true);
    assert_eq!(poll(&mut watch), Poll::Pending);

    // When
    page.fire();
    let polled = poll(&mut watch);

    // Then
    assert_eq!(polled, Poll::Ready(()));
    assert_eq!(*outcome.borrow(), Outcome::Caught(timed_out()));
    assert_eq!(woken(&count), 1);
}

#[test]
fn a_work_done_that_never_comes_is_refused_at_its_own_deadline() {
    // Given
    let (count, waker) = counting();
    let (late, work, page) = (Late::default(), Reported::default(), FakePage::default());
    let (mut watch, outcome) = racing(&late, &work, &page, &waker);

    // When
    page.fire();
    let polled = poll(&mut watch);

    // Then
    assert_eq!(page.armed(), [WORK_DONE_DEADLINE]);
    assert_eq!(polled, Poll::Ready(()));
    assert_eq!(*outcome.borrow(), Outcome::Caught(work_timed_out()));
    assert!(work_timed_out().contains("30000 ms") && work_timed_out().contains("1000 ms"));
    assert_eq!(woken(&count), 1);
}

#[test]
fn a_page_hidden_during_the_work_done_wait_rearms_it() {
    // Given
    let (count, waker) = counting();
    let (late, work, page) = (Late::default(), Reported::default(), FakePage::default());
    let (mut watch, outcome) = racing(&late, &work, &page, &waker);

    // When
    page.hidden.set(true);
    page.fire();
    let polled = poll(&mut watch);

    // Then
    assert_eq!(page.armed(), [WORK_DONE_DEADLINE, WORK_DONE_DEADLINE]);
    assert_eq!(polled, Poll::Pending);
    assert_eq!(*outcome.borrow(), Outcome::Waiting);
    assert_eq!(woken(&count), 0);
}

#[test]
fn a_page_hidden_during_the_scope_wait_after_work_done_rearms_it() {
    // Given
    let (count, waker) = counting();
    let (late, work, page) = (Late::default(), Reported::default(), FakePage::default());
    let (mut watch, outcome) = racing(&late, &work, &page, &waker);
    work.0.set(true);
    let _ = poll(&mut watch);

    // When
    page.hidden.set(true);
    page.fire();
    let polled = poll(&mut watch);

    // Then
    assert_eq!(page.armed(), [WORK_DONE_DEADLINE, DEADLINE, DEADLINE]);
    assert_eq!(polled, Poll::Pending);
    assert_eq!(*outcome.borrow(), Outcome::Waiting);
    assert_eq!(woken(&count), 0);
}
