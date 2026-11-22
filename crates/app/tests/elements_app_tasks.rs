//! Tasks through the app, with no window (async-components.md T2): a
//! wake polls the woken tasks before any frame, and asks for a frame
//! only when the poll changed the document or a task ended. The loop
//! the hook wakes is one detached platform (`waking`), whose task wakes
//! (`take_task_wake`) are apart from its frames, and the frames the app
//! asks for on a wake are read from another (`asked`).

use std::cell::{Cell, RefCell};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

use vapors_app::ElementsApp;
use vapors_dom::Dom;
use vapors_elements::Document;
use vapors_platform::{App, Event, Frame, Next, Platform};
use vapors_signals::{Deadline, Effect, Signal, spawn};
use vapors_text::FontSystem;

/// The app, its document's own first change taken, and the platform
/// whose waker the executor's hook calls, already given to the app.
struct Rig {
    app: ElementsApp,
    document: Rc<RefCell<Document>>,
    waking: Platform,
    asked: Platform,
}

fn rig() -> Rig {
    let document = Rc::new(RefCell::new(Document::new(FontSystem::bundle_only())));
    let mut app = ElementsApp::new(Rc::clone(&document));
    document.borrow_mut().take_changed();
    let waking = Platform::detached();
    app.drawn(&waking);
    let _ = waking.take_task_wake();
    Rig {
        app,
        document,
        waking,
        asked: Platform::detached(),
    }
}

/// A future that pends on every poll, counting them, and on its first
/// poll runs `first`.
struct Pends<F: FnMut()> {
    polls: Rc<Cell<u32>>,
    first: F,
}

impl<F: FnMut() + Unpin> Future for Pends<F> {
    type Output = Result<(), String>;

    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        self.polls.set(self.polls.get() + 1);
        if self.polls.get() == 1 {
            (self.first)();
        }
        Poll::Pending
    }
}

fn pends<F: FnMut() + Unpin>(first: F) -> (Rc<Cell<u32>>, Pends<F>) {
    let polls = Rc::new(Cell::new(0));
    let future = Pends {
        polls: Rc::clone(&polls),
        first,
    };
    (polls, future)
}

/// A text node in the body showing `count`.
fn shown(document: &Rc<RefCell<Document>>, count: Signal<u32>) {
    let text = {
        let mut doc = document.borrow_mut();
        let body = doc.body();
        let text = doc.create_text("0");
        doc.append(body, text);
        text
    };
    let shown = Rc::clone(document);
    // an effect runs on after its handle is dropped
    let _ = Effect::new(move || {
        let value = count.get().to_string();
        shown.borrow_mut().set_text(text, &value);
    });
    document.borrow_mut().take_changed();
}

#[test]
fn a_spawn_wakes_the_loop_through_the_hook_the_app_installed() {
    // Given
    let rig = rig();

    // When
    spawn(Deadline::User, pends(|| {}).1);

    // Then: the hook posted a task wake to the loop, not a frame
    assert!(rig.waking.take_task_wake(), "the spawn woke no loop");
    assert!(
        !rig.waking.take_redraw_request(),
        "the spawn asked for a frame"
    );
    assert!(!rig.asked.take_redraw_request());
}

#[test]
fn a_wake_whose_poll_changes_nothing_asks_for_no_frame() {
    // Given: a task whose poll writes nothing
    let mut rig = rig();
    let (polls, future) = pends(|| {});
    spawn(Deadline::User, future);

    // When
    rig.app.tasks_woken(&rig.asked);

    // Then: polled once, before any frame, and no frame asked for
    assert_eq!(polls.get(), 1);
    assert!(
        !rig.asked.take_redraw_request(),
        "a quiet poll asked for a frame"
    );
}

#[test]
fn a_wake_whose_poll_changes_the_document_asks_for_one_frame() {
    // Given: a task whose first poll writes a shown signal
    let mut rig = rig();
    let count = Signal::new(0_u32);
    shown(&rig.document, count);
    spawn(Deadline::User, pends(move || count.set(1)).1);

    // When
    rig.app.tasks_woken(&rig.asked);

    // Then
    assert!(
        rig.asked.take_redraw_request(),
        "the change asked for no frame"
    );
}

#[test]
fn a_wake_with_no_task_ready_polls_nothing_and_asks_for_no_frame() {
    // Given: a task polled once and pending, not woken again
    let mut rig = rig();
    let (polls, future) = pends(|| {});
    spawn(Deadline::User, future);
    rig.app.tasks_woken(&rig.asked);

    // When: a second wake, from something else
    rig.app.tasks_woken(&rig.asked);

    // Then
    assert_eq!(polls.get(), 1, "a pending task was polled without a wake");
    assert!(!rig.asked.take_redraw_request());
}

#[test]
fn a_task_that_completes_asks_for_a_frame() {
    // Given: a task that finishes on its first poll and writes nothing
    let mut rig = rig();
    spawn(Deadline::User, async { Ok::<(), String>(()) });

    // When
    rig.app.tasks_woken(&rig.asked);

    // Then: plan section 3.3 shows an ended task with a frame
    assert!(rig.asked.take_redraw_request());
}

#[test]
fn a_spawn_before_the_loop_is_known_wakes_the_loop_once_it_is() {
    // Given: an app that has seen no platform yet
    let document = Rc::new(RefCell::new(Document::new(FontSystem::bundle_only())));
    let mut app = ElementsApp::new(document);
    let (polls, future) = pends(|| {});
    spawn(Deadline::User, future);

    // When: the loop's platform reaches the app
    let waking = Platform::detached();
    app.drawn(&waking);

    // Then: the wake missed before the loop was known is posted now
    assert!(
        waking.take_task_wake(),
        "the early spawn never woke the loop"
    );
    app.tasks_woken(&waking);
    assert_eq!(polls.get(), 1);
}

#[test]
fn two_apps_on_one_thread_share_the_one_hook() {
    // Given: the executor is per thread, and so is its hook
    let document = Rc::new(RefCell::new(Document::new(FontSystem::bundle_only())));
    let _first = ElementsApp::new(Rc::clone(&document));

    // When: a second app is made on the same thread
    let _second = ElementsApp::new(document);

    // Then: no "set_wake_hook called twice" panic, and a spawn still works
    spawn(Deadline::User, pends(|| {}).1);
}

/// An app that keeps the default `tasks_woken`.
struct Plain;

impl App for Plain {
    fn event(&mut self, _: Event, _: &Platform) {}

    fn frame(&mut self, _: &Frame<'_>) -> Next {
        unreachable!("no frame is drawn without a window")
    }
}

#[test]
fn an_app_that_keeps_the_default_tasks_woken_asks_for_a_frame_per_wake() {
    // Given
    let platform = Platform::detached();

    // When
    Plain.tasks_woken(&platform);

    // Then: what every wake did before T2
    assert!(platform.take_redraw_request());
}
