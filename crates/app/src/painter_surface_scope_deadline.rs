//! A GPU error scope's deadline on the web, counting only time with the
//! page visible (`DEADLINE`). The page is read through `Page`, so a
//! native test can drive the timer and the page's visibility.

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use super::{DEADLINE, timed_out};

/// A page timer: resolves `Ok` when its time passes, or `Err` at once,
/// naming why, if the page would not set it.
pub(super) type Alarm = Pin<Box<dyn Future<Output = Result<(), String>>>>;

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
    loop {
        match visible_period(&page).await {
            Ok(true) => return timed_out(),
            Ok(false) => {}
            Err(why) => return why,
        }
    }
}

/// Waits one `DEADLINE` period: whether the page was visible at both
/// ends, or the refusal if the timer or the page's visibility failed.
async fn visible_period(page: &impl Page) -> Result<bool, String> {
    let hidden_when_armed = hidden(page)?;
    page.alarm(DEADLINE)
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
