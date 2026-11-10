use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll, Wake, Waker};
use std::time::Duration;

use vapors_platform::{Instant, Next};

use super::{DEADLINE, Scopes, Turn};
use crate::FrameDecision;

/// Counts its wakes, as the app's waker would ask the loop for frames.
#[derive(Default)]
struct Wakes(AtomicUsize);

impl Wake for Wakes {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

fn counting() -> (Arc<Wakes>, Waker) {
    let wakes = Arc::new(Wakes::default());
    (Arc::clone(&wakes), Waker::from(Arc::clone(&wakes)))
}

fn woken(wakes: &Wakes) -> usize {
    wakes.0.load(Ordering::SeqCst)
}

/// A scope whose result the test gives, waking whoever polled it, as a
/// WebGPU popErrorScope promise resolving does.
#[derive(Clone, Default)]
struct Late(Rc<RefCell<LateState>>);

#[derive(Default)]
struct LateState {
    /// `Some` once resolved: `Some(None)` is a scope that caught nothing.
    #[allow(clippy::option_option)] // a scope's own output, once it has one
    result: Option<Option<wgpu::Error>>,
    polled_by: Option<Waker>,
}

impl Late {
    fn resolve(&self, error: Option<&str>) {
        let mut state = self.0.borrow_mut();
        state.result = Some(error.map(|e| wgpu::Error::Validation {
            source: e.into(),
            description: e.to_owned(),
        }));
        state.polled_by.take().into_iter().for_each(Waker::wake);
    }
}

impl Future for Late {
    type Output = Option<wgpu::Error>;
    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self.0.borrow_mut();
        if let Some(result) = state.result.take() {
            return Poll::Ready(result);
        }
        state.polled_by = Some(context.waker().clone());
        Poll::Pending
    }
}

fn turn(waker: &Waker, now: Instant) -> Turn<'_> {
    Turn { waker, now }
}

/// Scopes holding one `Late` scope begun at `start`, polled once.
fn waiting_on(late: &Late, waker: &Waker, start: Instant) -> Scopes {
    let mut scopes = Scopes::default();
    scopes.wait_for(Box::pin(late.clone()), turn(waker, start));
    assert!(scopes.check(turn(waker, start)).is_ok());
    scopes
}

#[test]
fn a_pending_scope_gives_its_deadline_for_the_loop_to_wait_until() {
    // Given
    let (_, waker) = counting();
    let start = Instant::now();

    // When
    let scopes = waiting_on(&Late::default(), &waker, start);

    // Then
    assert_eq!(scopes.deadline(), Some(start + DEADLINE));
}

#[test]
fn a_scope_resolving_wakes_the_loop_once_and_its_frame_clears_it() {
    // Given
    let (count, waker) = counting();
    let start = Instant::now();
    let late = Late::default();
    let mut scopes = waiting_on(&late, &waker, start);

    // When
    late.resolve(None);
    let checked = scopes.check(turn(&waker, start + Duration::from_millis(1)));

    // Then
    assert_eq!(woken(&count), 1);
    assert!(checked.is_ok());
    assert_eq!(scopes.deadline(), None);
}

#[test]
fn an_error_in_a_late_result_refuses_the_woken_frame() {
    // Given
    let (count, waker) = counting();
    let start = Instant::now();
    let late = Late::default();
    let mut scopes = waiting_on(&late, &waker, start);

    // When
    late.resolve(Some("bad bind group"));
    let checked = scopes.check(turn(&waker, start + Duration::from_millis(1)));

    // Then
    assert_eq!(woken(&count), 1);
    let refusal = format!("{:?}", checked.expect_err("the late error refuses"));
    assert!(refusal.contains("bad bind group"), "{refusal}");
}

#[test]
fn a_scope_unresolved_at_the_deadline_refuses_with_no_wake_before() {
    // Given
    let (count, waker) = counting();
    let start = Instant::now();
    let mut scopes = waiting_on(&Late::default(), &waker, start);
    let just_before = start + DEADLINE.saturating_sub(Duration::from_millis(1));

    // When
    let before = scopes.check(turn(&waker, just_before));
    let at = scopes.check(turn(&waker, start + DEADLINE));
    let after = scopes.check(turn(&waker, start + DEADLINE * 2));

    // Then
    assert_eq!(woken(&count), 0);
    assert!(before.is_ok());
    let at = format!("{:?}", at.expect_err("refused at the deadline"));
    assert!(at.contains("within 1000 ms"), "{at}");
    assert!(after.is_err());
    assert_eq!(scopes.deadline(), None);
}

/// What an idle app's loop does after a frame of `scopes`.
fn idle_next(scopes: &Scopes) -> Next {
    FrameDecision::default().next(scopes.deadline())
}

#[test]
fn a_scope_resolving_early_leaves_no_frame_at_its_old_deadline() {
    // Given
    let (_, waker) = counting();
    let start = Instant::now();
    let late = Late::default();
    let mut scopes = waiting_on(&late, &waker, start);
    let asked = idle_next(&scopes);

    // When
    late.resolve(None);
    let woken_frame = scopes.check(turn(&waker, start + Duration::from_millis(1)));

    // Then
    assert_eq!(asked, Next::WaitUntil(start + DEADLINE));
    assert!(woken_frame.is_ok());
    assert_eq!(idle_next(&scopes), Next::Wait);
}

#[test]
fn a_scope_never_resolving_gets_one_frame_at_its_deadline_which_refuses() {
    // Given
    let (count, waker) = counting();
    let start = Instant::now();
    let mut scopes = waiting_on(&Late::default(), &waker, start);
    let asked = idle_next(&scopes);

    // When
    let deadline_frame = scopes.check(turn(&waker, start + DEADLINE));

    // Then
    assert_eq!(asked, Next::WaitUntil(start + DEADLINE));
    assert_eq!(woken(&count), 0);
    assert!(deadline_frame.is_err());
    assert_eq!(idle_next(&scopes), Next::Wait);
}
