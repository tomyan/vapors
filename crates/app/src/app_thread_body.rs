//! What runs on the application thread: the runtime, the executor's wake
//! hook, the app side, and the loop of turns.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, mpsc};

use tokio::runtime::Runtime;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use vapors_signals::{run_ready, set_wake_hook};

/// What reaches the loop.
pub(super) enum Turn<M> {
    Message(M),
    /// A task was woken: poll the woken tasks.
    Wake,
    Stop,
}

/// The thread's answer to `start`: ready, or why its runtime failed.
pub(super) type Ready = Result<(), String>;

pub(super) struct Body<M> {
    receiver: UnboundedReceiver<Turn<M>>,
    waker: UnboundedSender<Turn<M>>,
    turns: Arc<AtomicU64>,
    /// True from a wake sent until the turn that polls for it.
    wake_pending: Arc<AtomicBool>,
    report: fn(&str),
}

impl<M: Send + 'static> Body<M> {
    pub(super) fn new(
        receiver: UnboundedReceiver<Turn<M>>,
        waker: UnboundedSender<Turn<M>>,
        turns: Arc<AtomicU64>,
        report: fn(&str),
    ) -> Self {
        let wake_pending = Arc::default();
        Body {
            receiver,
            waker,
            turns,
            wake_pending,
            report,
        }
    }

    /// The thread's whole life. The app side is built inside the
    /// runtime's context, so it may make Tokio timers.
    pub(super) fn run<H: FnMut(M)>(self, make: impl FnOnce() -> H, ready: &mpsc::Sender<Ready>) {
        let runtime = match runtime() {
            Ok(runtime) => runtime,
            // Err: start gave up waiting; its deadline error is the report
            Err(error) => return drop(ready.send(Err(error))),
        };
        self.install_hook();
        let handler = {
            let _context = runtime.enter();
            make()
        };
        // Err: start gave up waiting and refused; the thread stops at once
        let _ = ready.send(Ok(()));
        runtime.block_on(self.serve(handler));
    }

    /// Every wake and spawn on this thread sends one `Wake`, until the
    /// turn that polls for it.
    fn install_hook(&self) {
        let (waker, pending) = (self.waker.clone(), Arc::clone(&self.wake_pending));
        set_wake_hook(move || {
            if !pending.swap(true, Ordering::SeqCst) {
                // Err: the thread has ended, so no task can be polled again
                let _ = waker.send(Turn::Wake);
            }
        });
    }

    async fn serve(mut self, mut handler: impl FnMut(M)) {
        while let Some(turn) = self.receiver.recv().await {
            self.turns.fetch_add(1, Ordering::SeqCst);
            match turn {
                Turn::Message(message) => handler(message),
                Turn::Wake => self.poll(),
                Turn::Stop => return,
            }
        }
    }

    fn poll(&self) {
        self.wake_pending.store(false, Ordering::SeqCst);
        for failure in run_ready().failures {
            (self.report)(&failure);
        }
    }
}

fn runtime() -> Result<Runtime, String> {
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .map_err(|error| {
            format!("the application thread's Tokio runtime could not be built: {error}")
        })
}
