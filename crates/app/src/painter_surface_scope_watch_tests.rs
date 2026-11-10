use std::cell::{Cell, RefCell};
use std::future::{Future, ready};
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use super::super::fakes::{Late, counting, woken};
use super::super::timed_out;
use super::{LoopWaker, Outcome, Watch, Watched};

/// A deadline the test passes, which says when it is dropped, as the
/// web's timer is cleared.
#[derive(Clone, Default)]
struct FakeDeadline {
    passed: Rc<Cell<bool>>,
    dropped: Rc<Cell<bool>>,
}

struct Pending(FakeDeadline);

impl Future for Pending {
    type Output = String;
    fn poll(self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<String> {
        if self.0.passed.get() {
            Poll::Ready(timed_out())
        } else {
            Poll::Pending
        }
    }
}

impl Drop for Pending {
    fn drop(&mut self) {
        self.0.dropped.set(true);
    }
}

fn error(text: &str) -> wgpu::Error {
    wgpu::Error::Validation {
        source: text.into(),
        description: text.to_owned(),
    }
}

/// A watch on `late` racing `deadline`, as the page's task queue drives
/// one: polled once, and again after the scope resolves or the
/// deadline passes.
fn racing(
    late: &Late,
    deadline: &FakeDeadline,
    waker: &LoopWaker,
) -> (Watch, Rc<RefCell<Outcome>>) {
    let outcome = Rc::new(RefCell::new(Outcome::Waiting));
    let deadline = Box::pin(Pending(deadline.clone()));
    let mut watch = Watch::new(Box::pin(late.clone()), deadline, &outcome, waker);
    assert_eq!(poll(&mut watch), Poll::Pending);
    (watch, outcome)
}

fn watch(late: &Late, waker: &LoopWaker) -> (Watch, Rc<RefCell<Outcome>>) {
    racing(late, &FakeDeadline::default(), waker)
}

fn poll(watch: &mut Watch) -> Poll<()> {
    Pin::new(watch).poll(&mut Context::from_waker(Waker::noop()))
}

#[test]
fn a_clean_result_before_the_deadline_wakes_no_loop_and_clears_the_deadline() {
    // Given
    let (count, waker) = counting();
    let (late, deadline) = (Late::default(), FakeDeadline::default());
    let (mut watch, outcome) = racing(&late, &deadline, &waker);

    // When
    late.resolve(None);
    let polled = poll(&mut watch);
    drop(watch);

    // Then
    assert_eq!(polled, Poll::Ready(()));
    assert_eq!(*outcome.borrow(), Outcome::Clean);
    assert_eq!(woken(&count), 0);
    assert!(deadline.dropped.get());
}

#[test]
fn a_deadline_passing_first_wakes_the_loop_once_and_records_the_timeout() {
    // Given
    let (count, waker) = counting();
    let (late, deadline) = (Late::default(), FakeDeadline::default());
    let (mut watch, outcome) = racing(&late, &deadline, &waker);

    // When
    deadline.passed.set(true);
    let polled = poll(&mut watch);

    // Then
    assert_eq!(polled, Poll::Ready(()));
    assert_eq!(*outcome.borrow(), Outcome::Caught(timed_out()));
    assert_eq!(woken(&count), 1);
}

#[test]
fn a_watched_scope_resolving_with_an_error_wakes_the_loop_once() {
    // Given
    let (count, waker) = counting();
    let late = Late::default();
    let (mut watch, outcome) = watch(&late, &waker);

    // When
    late.resolve(Some("bad bind group"));
    let polled = poll(&mut watch);

    // Then
    assert_eq!(polled, Poll::Ready(()));
    assert!(matches!(&*outcome.borrow(), Outcome::Caught(e) if e.contains("bad bind group")));
    assert_eq!(woken(&count), 1);
}

#[test]
fn the_waker_given_after_a_scope_began_is_the_one_its_error_wakes() {
    // Given
    let waker = LoopWaker::default();
    let late = Late::default();
    let (mut watch, _) = watch(&late, &waker);
    let (count, given) = counting();

    // When
    *waker.borrow_mut() = given.borrow_mut().take();
    late.resolve(Some("bad bind group"));
    let _ = poll(&mut watch);

    // Then
    assert_eq!(woken(&count), 1);
}

#[test]
fn an_error_with_no_waker_given_yet_is_kept_and_wakes_nothing() {
    // Given
    let late = Late::default();
    let (mut watch, outcome) = watch(&late, &LoopWaker::default());

    // When
    late.resolve(Some("bad bind group"));
    let polled = poll(&mut watch);

    // Then
    assert_eq!(polled, Poll::Ready(()));
    assert!(matches!(&*outcome.borrow(), Outcome::Caught(_)));
}

#[test]
fn a_scope_resolved_when_popped_is_read_at_once_and_wakes_nothing() {
    // Given
    let (count, waker) = counting();
    let scope = Box::pin(ready(Some(error("bad bind group"))));

    // When
    let mut watched = Watched::new(scope, &waker);

    // Then
    assert!(matches!(watched.outcome(), Outcome::Caught(_)));
    assert_eq!(woken(&count), 0);
}

#[test]
fn natively_an_unresolved_scope_is_read_on_a_later_outcome() {
    // Given
    let (_, waker) = counting();
    let late = Late::default();
    let mut watched = Watched::new(Box::pin(late.clone()), &waker);
    let before = watched.outcome();

    // When
    late.resolve(None);

    // Then
    assert_eq!(before, Outcome::Waiting);
    assert_eq!(watched.outcome(), Outcome::Clean);
}
