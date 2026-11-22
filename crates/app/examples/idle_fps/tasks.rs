//! The task cases (`docs/plans/async-components.md` T2): a document
//! showing a count, and one spawned task that the thread below wakes,
//! or not, and that writes the count, or not.

use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use vapors_app::ElementsApp;
use vapors_dom::Dom;
use vapors_elements::Document;
use vapors_signals::{Deadline, Effect, Signal, spawn};
use vapors_text::FontSystem;

/// How often the task cases' thread wakes the task.
const WAKE_EVERY: Duration = Duration::from_millis(500);

/// The fewest wakes a waking task case passes with: the thread wakes
/// only a task polled since its last wake, so two show that a task wake
/// was polled at least once after the first poll (5 in 3 s here).
pub const MIN_WAKES: u64 = 2;

/// The task cases whose task is woken by the thread.
pub const WAKING: [&str; 2] = ["task-quiet", "task-change"];

/// A task that pends on every poll, keeping its waker for the thread;
/// from its second poll it writes the poll count to `count`, if given.
/// Its first poll starts the waking thread, so every wake falls after
/// the first frame and inside the count.
struct Woken {
    waker: Arc<Mutex<Option<Waker>>>,
    polls: u64,
    count: Option<Signal<u64>>,
    start_waking: Option<Arc<AtomicU64>>,
}

impl Future for Woken {
    type Output = Result<(), String>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.polls += 1;
        *self.waker.lock().expect("the waker's lock") = Some(cx.waker().clone());
        if let Some(count) = self.count
            && self.polls > 1
        {
            count.set(self.polls);
        }
        if let Some(wakes) = self.start_waking.take() {
            wake_every(Arc::clone(&self.waker), wakes);
        }
        Poll::Pending
    }
}

fn wake_every(kept: Arc<Mutex<Option<Waker>>>, wakes: Arc<AtomicU64>) {
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(WAKE_EVERY);
            if let Some(waker) = kept.lock().expect("the waker's lock").take() {
                wakes.fetch_add(1, Ordering::SeqCst);
                waker.wake();
            }
        }
    });
}

/// The app for a task case, or `None` for another name.
pub fn task_app(name: &str, wakes: &Arc<AtomicU64>) -> Option<ElementsApp> {
    let (woken, writes) = match name {
        "task-pending" => (false, false),
        "task-quiet" => (true, false),
        "task-change" => (true, true),
        _ => return None,
    };
    let document = Rc::new(RefCell::new(Document::new(FontSystem::new())));
    let count = Signal::new(0_u64);
    show(&document, count);
    // the hook is installed here; a spawn before it is refused
    let app = ElementsApp::new(document);
    // Deadline::User: the task runs until the window is closed
    spawn(
        Deadline::User,
        Woken {
            waker: Arc::default(),
            polls: 0,
            count: writes.then_some(count),
            start_waking: woken.then(|| Arc::clone(wakes)),
        },
    );
    Some(app)
}

/// A text in the body showing `count`.
fn show(document: &Rc<RefCell<Document>>, count: Signal<u64>) {
    let mut doc = document.borrow_mut();
    let body = doc.body();
    let text = doc.create_text("0");
    doc.append(body, text);
    drop(doc);
    let document = Rc::clone(document);
    // an effect runs on after its handle is dropped
    let _ = Effect::new(move || {
        let value = count.get().to_string();
        document.borrow_mut().set_text(text, &value);
    });
}
