//! One screenshot in a browser context of its own
//! (`Target.createBrowserContext`): nothing a page stores (localStorage,
//! `IndexedDB`, cookies, service workers, the HTTP cache) reaches the next
//! screenshot. The context is disposed after it, however it ends.
//!
//! Two settings stand in for the one-shot launch's flags, each measured
//! needed on 2026-11-22 (docs/perf/2026-11-22-screenshot-browser-reuse.md
//! on the experiment branch):
//! - the viewport, `Emulation.setDeviceMetricsOverride` at the window
//!   size: a new target's viewport was the window less 87 rows (640x313
//!   for 640x400), where `--screenshot` captures the whole window size;
//! - `--virtual-time-budget=5000`: virtual time paused, the navigation
//!   started, then a 5000 ms budget (`pauseIfNetworkFetchesPending`), and
//!   the screenshot once it has expired. A budget set before navigating
//!   expired at once on the idle blank page, and the screenshot never came.

use std::time::Instant;

use serde_json::{Value, json};
use vapors_devtools::Session;

/// The one-shot launch's `--virtual-time-budget`, in milliseconds.
pub(super) const VIRTUAL_TIME_BUDGET_MS: u32 = 5000;

/// The PNG of `url` at `size` CSS pixels in a new context of the browser
/// behind `browser`, whose `DevTools` port is `port`, all by `until`.
///
/// # Errors
/// A call refused or unanswered by `until`, naming it; the disposal's
/// failure too.
pub(super) fn screenshot(
    browser: &mut Session,
    port: u16,
    url: &str,
    size: [u32; 2],
    until: Instant,
) -> Result<Vec<u8>, String> {
    let made = call(
        browser,
        "Target.createBrowserContext",
        &json!({ "disposeOnDetach": true }),
        until,
    )?;
    let context = text(&made, "/browserContextId", "Target.createBrowserContext")?;
    let shot = in_context(browser, port, &context, url, size, until);
    let disposed = call(
        browser,
        "Target.disposeBrowserContext",
        &json!({ "browserContextId": context }),
        until,
    );
    match (shot, disposed) {
        (shot, Ok(_)) => shot,
        (Ok(_), Err(why)) => Err(why),
        (Err(failure), Err(why)) => Err(format!("{failure}\nand {why}")),
    }
}

fn in_context(
    browser: &mut Session,
    port: u16,
    context: &str,
    url: &str,
    [width, height]: [u32; 2],
    until: Instant,
) -> Result<Vec<u8>, String> {
    let params = json!({ "url": "about:blank", "browserContextId": context, "width": width, "height": height });
    let target = call(browser, "Target.createTarget", &params, until)?;
    let id = text(&target, "/targetId", "Target.createTarget")?;
    let socket = format!("ws://127.0.0.1:{port}/devtools/page/{id}");
    let mut page = Session::connect(&socket, until.saturating_duration_since(Instant::now()))?;
    let metrics =
        json!({ "width": width, "height": height, "deviceScaleFactor": 0, "mobile": false });
    call(
        &mut page,
        "Emulation.setDeviceMetricsOverride",
        &metrics,
        until,
    )?;
    settle(&mut page, url, until)?;
    let shot = call(
        &mut page,
        "Page.captureScreenshot",
        &json!({ "format": "png" }),
        until,
    )?;
    super::chromium_base64::decode(&text(&shot, "/data", "Page.captureScreenshot")?)
}

/// Navigate to `url` and let its virtual-time budget run out.
fn settle(page: &mut Session, url: &str, until: Instant) -> Result<(), String> {
    let budget =
        json!({ "policy": "pauseIfNetworkFetchesPending", "budget": VIRTUAL_TIME_BUDGET_MS });
    call(
        page,
        "Emulation.setVirtualTimePolicy",
        &json!({ "policy": "pause" }),
        until,
    )?;
    let navigated = call(page, "Page.navigate", &json!({ "url": url }), until)?;
    if let Some(error) = navigated.get("errorText").and_then(Value::as_str) {
        return Err(format!(
            "{url} did not load ({error}), so the screenshot would be Chromium's error page. \
             Fix: write the page first; reproduce: ls -l '{}'",
            url.trim_start_matches("file://")
        ));
    }
    call(page, "Emulation.setVirtualTimePolicy", &budget, until)?;
    let left = until.saturating_duration_since(Instant::now());
    page.wait_event("Emulation.virtualTimeBudgetExpired", left)
        .map(drop)
}

fn call(
    session: &mut Session,
    method: &str,
    params: &Value,
    until: Instant,
) -> Result<Value, String> {
    session.call(
        method,
        params,
        until.saturating_duration_since(Instant::now()),
    )
}

fn text(answer: &Value, pointer: &str, method: &str) -> Result<String, String> {
    answer
        .pointer(pointer)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("{method} answered {answer} with no text at {pointer}"))
}
