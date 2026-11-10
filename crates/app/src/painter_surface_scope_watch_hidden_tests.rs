//! A watched scope's deadline counting visible time only: the page's
//! timers and visibility are driven by the test (`FakePage`).

use std::cell::{Cell, RefCell};
use std::future::{Future, ready};
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use super::super::deadline::{Alarm, Page, visible_deadline};
use super::super::fakes::{Late, counting, woken};
use super::super::{DEADLINE, timed_out};
use super::{LoopWaker, Outcome, Watch};

/// A page whose visibility the test sets and whose latest timer it
/// fires; every timer asked for is recorded.
#[derive(Clone, Default)]
struct FakePage {
    hidden: Rc<Cell<bool>>,
    unreadable: Rc<Cell<bool>>,
    alarms: Rc<RefCell<Vec<(Duration, Rc<Cell<bool>>)>>>,
}

impl FakePage {
    fn fire(&self) {
        let alarms = self.alarms.borrow();
        alarms.last().expect("a timer is armed").1.set(true);
    }

    fn armed(&self) -> Vec<Duration> {
        self.alarms
            .borrow()
            .iter()
            .map(|(after, _)| *after)
            .collect()
    }
}

impl Page for FakePage {
    fn alarm(&self, after: Duration) -> Alarm {
        let fired = Rc::new(Cell::new(false));
        self.alarms.borrow_mut().push((after, Rc::clone(&fired)));
        Box::pin(Fired(fired))
    }

    fn hidden(&self) -> Result<bool, String> {
        if self.unreadable.get() {
            return Err("no document".to_owned());
        }
        Ok(self.hidden.get())
    }
}

/// A fake timer: resolves once the test fires it.
struct Fired(Rc<Cell<bool>>);

impl Future for Fired {
    type Output = Result<(), String>;
    fn poll(self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<Self::Output> {
        if self.0.get() {
            Poll::Ready(Ok(()))
        } else {
            Poll::Pending
        }
    }
}

/// A watch on `late` racing a visible-time deadline on `page`, polled
/// once, as the page's task queue drives one.
fn racing(late: &Late, page: &FakePage, waker: &LoopWaker) -> (Watch, Rc<RefCell<Outcome>>) {
    let outcome = Rc::new(RefCell::new(Outcome::Waiting));
    let deadline = Box::pin(visible_deadline(page.clone()));
    let mut watch = Watch::new(Box::pin(late.clone()), deadline, &outcome, waker);
    assert_eq!(poll(&mut watch), Poll::Pending);
    (watch, outcome)
}

fn poll(watch: &mut Watch) -> Poll<()> {
    Pin::new(watch).poll(&mut Context::from_waker(Waker::noop()))
}

#[test]
fn a_deadline_passing_while_hidden_is_rearmed_and_a_clean_result_after_refuses_nothing() {
    // Given
    let (count, waker) = counting();
    let (late, page) = (Late::default(), FakePage::default());
    let (mut watch, outcome) = racing(&late, &page, &waker);

    // When
    page.hidden.set(true);
    page.fire();
    let while_hidden = poll(&mut watch);
    page.hidden.set(false);
    late.resolve(None);

    // Then
    assert_eq!(while_hidden, Poll::Pending);
    assert_eq!(page.armed(), [DEADLINE, DEADLINE]);
    assert_eq!(poll(&mut watch), Poll::Ready(()));
    assert_eq!(*outcome.borrow(), Outcome::Clean);
    assert_eq!(woken(&count), 0);
}

#[test]
fn a_scope_unresolved_through_a_whole_visible_period_after_hiding_is_refused() {
    // Given
    let (count, waker) = counting();
    let (late, page) = (Late::default(), FakePage::default());
    let (mut watch, outcome) = racing(&late, &page, &waker);
    page.hidden.set(true);
    page.fire();
    let _ = poll(&mut watch);
    page.hidden.set(false);

    // When
    page.fire();
    let armed_hidden = poll(&mut watch);
    page.fire();
    let visible_throughout = poll(&mut watch);

    // Then
    assert_eq!(armed_hidden, Poll::Pending);
    assert_eq!(visible_throughout, Poll::Ready(()));
    assert_eq!(*outcome.borrow(), Outcome::Caught(timed_out()));
    assert_eq!(woken(&count), 1);
}

#[test]
fn a_period_visible_at_both_ends_refuses_when_it_passes() {
    // Given
    let (count, waker) = counting();
    let (late, page) = (Late::default(), FakePage::default());
    let (mut watch, outcome) = racing(&late, &page, &waker);

    // When
    page.fire();

    // Then
    assert_eq!(poll(&mut watch), Poll::Ready(()));
    assert_eq!(page.armed(), [DEADLINE]);
    assert_eq!(*outcome.borrow(), Outcome::Caught(timed_out()));
    assert_eq!(woken(&count), 1);
}

#[test]
fn a_page_whose_visibility_cannot_be_read_refuses_naming_why() {
    // Given
    let page = FakePage::default();
    page.unreadable.set(true);
    let outcome = Rc::new(RefCell::new(Outcome::Waiting));
    let deadline = Box::pin(visible_deadline(page));
    let mut watch = Watch::new(Box::pin(Late::default()), deadline, &outcome, &counting().1);

    // When
    let polled = poll(&mut watch);

    // Then
    assert_eq!(polled, Poll::Ready(()));
    assert!(matches!(&*outcome.borrow(), Outcome::Caught(e) if e.contains("no document")));
}

#[test]
fn a_timer_the_page_will_not_set_refuses_naming_why() {
    // Given
    struct Refusing;
    impl Page for Refusing {
        fn alarm(&self, _after: Duration) -> Alarm {
            Box::pin(ready(Err("setTimeout refused".to_owned())))
        }
        fn hidden(&self) -> Result<bool, String> {
            Ok(false)
        }
    }
    let outcome = Rc::new(RefCell::new(Outcome::Waiting));
    let deadline = Box::pin(visible_deadline(Refusing));

    // When
    let mut watch = Watch::new(Box::pin(Late::default()), deadline, &outcome, &counting().1);

    // Then
    assert_eq!(poll(&mut watch), Poll::Ready(()));
    assert!(matches!(&*outcome.borrow(), Outcome::Caught(e) if e.contains("setTimeout refused")));
}

#[test]
fn a_deadline_passing_hidden_after_the_surface_unmounted_ends_the_watch_and_rearms_nothing() {
    // Given
    let (count, waker) = counting();
    let (late, page) = (Late::default(), FakePage::default());
    let (mut watch, outcome) = racing(&late, &page, &waker);

    // When
    drop(outcome);
    page.hidden.set(true);
    page.fire();

    // Then
    assert_eq!(poll(&mut watch), Poll::Ready(()));
    assert_eq!(page.armed(), [DEADLINE]);
    assert_eq!(woken(&count), 0);
}
