//! A watched scope's deadline starting when the GPU reports its work
//! done (`staged_deadline`, `chosen`, `work_done_from`): the page's
//! timers and the GPU's report are driven by the test.

use std::cell::{Cell, RefCell};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use super::hidden_tests::FakePage;
use super::{LoopWaker, Outcome, Watch};
use crate::painter::surfaces::scope::deadline::{chosen, work_done_from};
use crate::painter::surfaces::scope::fakes::{Late, counting, counting_waker, woken};
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

type Callback = Box<dyn FnOnce() + Send>;

/// A watch on `late` whose deadline starts at `work`'s report, polled
/// once, as the page's task queue drives one.
fn racing(
    late: &Late,
    work: &Reported,
    page: &FakePage,
    waker: &LoopWaker,
) -> (Watch, Rc<RefCell<Outcome>>) {
    let outcome = Rc::new(RefCell::new(Outcome::Waiting));
    let deadline = chosen(page.clone(), Some(Box::pin(work.clone())));
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

    // When: the report has not come yet, then it does, then the scope resolves
    let before_report = poll(&mut watch);
    let armed_before_report = page.armed();
    work.0.set(true);
    let reported = poll(&mut watch);
    late.resolve(None);
    let resolved = poll(&mut watch);

    // Then: until the report only the work-done wait was armed, never DEADLINE
    assert_eq!(armed_before_report, [WORK_DONE_DEADLINE]);
    assert_eq!(page.armed(), [WORK_DONE_DEADLINE, DEADLINE]);
    assert_eq!((before_report, reported), (Poll::Pending, Poll::Pending));
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

#[test]
fn without_work_the_deadline_runs_from_the_start() {
    // Given
    let page = FakePage::default();

    // When
    let mut deadline = chosen(page.clone(), None);
    let polled = deadline
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()));

    // Then
    assert!(polled.is_pending());
    assert_eq!(page.armed(), [DEADLINE]);
}

#[test]
fn a_report_arriving_through_the_registered_callback_resolves_the_work() {
    // Given: a register closure that keeps the callback, as a queue would
    let slot: Rc<RefCell<Option<Callback>>> = Rc::default();
    let kept = Rc::clone(&slot);
    let mut work = work_done_from(move |report| *kept.borrow_mut() = Some(report));
    let (count, waker) = counting_waker();
    let mut context = Context::from_waker(&waker);
    let before = work.as_mut().poll(&mut context);

    // When
    let report = slot
        .borrow_mut()
        .take()
        .expect("the callback was registered");
    report();

    // Then: the waiter was woken and the work reads done
    assert!(before.is_pending());
    assert_eq!(woken(&count), 1);
    assert_eq!(work.as_mut().poll(&mut context), Poll::Ready(()));
}

#[test]
fn a_report_made_before_the_first_poll_still_resolves_the_work() {
    // Given
    let mut work = work_done_from(|report| report());

    // When
    let polled = work.as_mut().poll(&mut Context::from_waker(Waker::noop()));

    // Then
    assert_eq!(polled, Poll::Ready(()));
}
