//! A GPU error scope's deadline on the web, counting only time with the
//! page visible (`DEADLINE`). The page is read through `Page`, so a
//! native test can drive the timer and the page's visibility.

use std::future::Future;
use std::pin::Pin;
use std::task::Poll;
use std::time::Duration;

use super::{DEADLINE, WORK_DONE_DEADLINE, WorkDone, timed_out, work_timed_out};

/// A page timer: resolves `Ok` when its time passes, or `Err` at once,
/// naming why, if the page would not set it.
pub(super) type Alarm = Pin<Box<dyn Future<Output = Result<(), String>>>>;

impl<P: Page> Page for &P {
    fn alarm(&self, after: Duration) -> Alarm {
        (**self).alarm(after)
    }

    fn hidden(&self) -> Result<bool, String> {
        (**self).hidden()
    }
}

/// What a deadline reads from the page.
pub(super) trait Page {
    /// A timer firing `after` from now.
    fn alarm(&self, after: Duration) -> Alarm;
    /// Whether the page is hidden, or why that cannot be read.
    fn hidden(&self) -> Result<bool, String>;
}

/// Resolves, with the refusal to record, once one whole `DEADLINE`
/// period has passed with the page visible both when it was armed and
/// when it fired. A period with the page hidden at either end is re-armed
/// for the full `DEADLINE`, not the remainder: without a listener for
/// visibility changes the timer cannot tell how much of that period was
/// visible, and refusing a working surface is the worse error. A scope is
/// so refused after at least `DEADLINE` and at most twice it of visible
/// time, unless the page was hidden and shown again within one period.
pub(super) async fn visible_deadline(page: impl Page) -> String {
    match visible_wait(&page, DEADLINE).await {
        Ok(()) => timed_out(),
        Err(why) => why,
    }
}

/// Waits for `work` (the GPU's report that what the scope covers is
/// done) for at most one `WORK_DONE_DEADLINE` of visible time, then
/// runs `visible_deadline` from that moment: the scope's deadline
/// measures the browser reporting an error, not the GPU drawing.
pub(super) async fn staged_deadline(page: impl Page, mut work: WorkDone) -> String {
    let mut wait = Box::pin(visible_wait(&page, WORK_DONE_DEADLINE));
    let first = std::future::poll_fn(|context| {
        if work.as_mut().poll(context).is_ready() {
            return Poll::Ready(None);
        }
        wait.as_mut().poll(context).map(Some)
    })
    .await;
    match first {
        None => visible_deadline(&page).await,
        Some(Ok(())) => work_timed_out(),
        Some(Err(why)) => why,
    }
}

/// Waits whole periods of `period` until one was visible at both ends:
/// `Ok`, or the refusal if the timer or the page's visibility failed.
async fn visible_wait(page: &impl Page, period: Duration) -> Result<(), String> {
    while !visible_period(page, period).await? {}
    Ok(())
}

/// Waits one `period`: whether the page was visible at both ends.
async fn visible_period(page: &impl Page, period: Duration) -> Result<bool, String> {
    let hidden_when_armed = hidden(page)?;
    page.alarm(period)
        .await
        .map_err(|why| format!("the GPU error scope's deadline timer was not set: {why}"))?;
    Ok(!hidden_when_armed && !hidden(page)?)
}

fn hidden(page: &impl Page) -> Result<bool, String> {
    page.hidden().map_err(|why| {
        format!("the GPU error scope's deadline could not read whether the page is hidden: {why}")
    })
}

/// The page: its `setTimeout` and `document.hidden`.
#[cfg(target_family = "wasm")]
pub(super) struct WebPage;

#[cfg(target_family = "wasm")]
impl Page for WebPage {
    fn alarm(&self, after: Duration) -> Alarm {
        Box::pin(vapors_platform::Timer::after(after))
    }

    fn hidden(&self) -> Result<bool, String> {
        vapors_platform::page_hidden()
    }
}

/// The GPU's report that the work submitted so far is done: wgpu's
/// `on_submitted_work_done`, which the web maps to `onSubmittedWorkDone`.
#[cfg(target_family = "wasm")]
pub(super) fn submitted_work_done(queue: &wgpu::Queue) -> WorkDone {
    use std::sync::{Arc, Mutex};
    use std::task::Waker;

    type Slot = Arc<Mutex<(bool, Option<Waker>)>>;
    let slot = Slot::default();
    let reported = Arc::clone(&slot);
    queue.on_submitted_work_done(move || {
        let mut state = reported.lock().expect("work-done slot");
        state.0 = true;
        state.1.take().into_iter().for_each(Waker::wake);
    });
    Box::pin(std::future::poll_fn(move |context| {
        let mut state = slot.lock().expect("work-done slot");
        if state.0 {
            return Poll::Ready(());
        }
        state.1 = Some(context.waker().clone());
        Poll::Pending
    }))
}
