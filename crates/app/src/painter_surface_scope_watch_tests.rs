use std::cell::RefCell;
use std::future::{Future, ready};
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use super::super::fakes::{Late, counting, woken};
use super::{LoopWaker, Outcome, Watch, Watched};

fn error(text: &str) -> wgpu::Error {
    wgpu::Error::Validation {
        source: text.into(),
        description: text.to_owned(),
    }
}

/// A watch on `late`, as the page's task queue drives one: polled once,
/// and again after the scope resolves.
fn watch(late: &Late, waker: &LoopWaker) -> (Watch, Rc<RefCell<Outcome>>) {
    let outcome = Rc::new(RefCell::new(Outcome::Waiting));
    let mut watch = Watch::new(Box::pin(late.clone()), &outcome, waker);
    assert_eq!(poll(&mut watch), Poll::Pending);
    (watch, outcome)
}

fn poll(watch: &mut Watch) -> Poll<()> {
    Pin::new(watch).poll(&mut Context::from_waker(Waker::noop()))
}

#[test]
fn a_watched_scope_resolving_with_no_error_wakes_no_loop() {
    // Given
    let (count, waker) = counting();
    let late = Late::default();
    let (mut watch, outcome) = watch(&late, &waker);

    // When
    late.resolve(None);
    let polled = poll(&mut watch);

    // Then
    assert_eq!(polled, Poll::Ready(()));
    assert_eq!(*outcome.borrow(), Outcome::Clean);
    assert_eq!(woken(&count), 0);
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
