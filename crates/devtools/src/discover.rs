//! Finding the page of a Chromium started with `--remote-debugging-port=0`:
//! the port it chose, from `DevToolsActivePort` in its profile, and the
//! page target's WebSocket, from `/json/list`. Both are polled by a
//! deadline while what they wait for is merely not there yet: the file
//! or its second line missing, the port not listening, no page yet.
//! Anything else is refused at once, with its own reason.

use std::io::ErrorKind;
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::discover_list::json_list;

const POLL: Duration = Duration::from_millis(50);

/// The port in `profile`'s `DevToolsActivePort` within `deadline`;
/// `alive` is asked on each poll, and its error stops the wait.
///
/// # Errors
/// As [`port_until`].
pub fn port(
    profile: &Path,
    deadline: Duration,
    alive: &mut dyn FnMut() -> Result<(), String>,
) -> Result<u16, String> {
    port_until(profile, Instant::now() + deadline, alive)
}

/// The port, once both lines (the port, the browser's path) are written.
///
/// # Errors
/// A file that cannot be read, a first line that is not a port, `alive`'s
/// error, or no whole file by `until`.
pub fn port_until(
    profile: &Path,
    until: Instant,
    alive: &mut dyn FnMut() -> Result<(), String>,
) -> Result<u16, String> {
    let path = profile.join("DevToolsActivePort");
    loop {
        if let Some(port) = read_port(&path)? {
            return Ok(port);
        }
        alive()?;
        if Instant::now() >= until {
            return Err(format!(
                "devtools: {} held no port by the deadline",
                path.display()
            ));
        }
        thread::sleep(POLL);
    }
}

/// The port, `None` while the file or its second line is not written.
fn read_port(path: &Path) -> Result<Option<u16>, String> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("devtools: {}: {e}", path.display())),
    };
    let mut lines = text.lines();
    let (Some(port), Some(_browser)) = (lines.next(), lines.next()) else {
        return Ok(None);
    };
    port.trim().parse().map(Some).map_err(|_| {
        format!(
            "devtools: {} names port `{port}`, not a number",
            path.display()
        )
    })
}

/// The first page target's WebSocket URL within `deadline`.
///
/// # Errors
/// As [`page_socket_until`].
pub fn page_socket(port: u16, deadline: Duration) -> Result<String, String> {
    page_socket_until(port, Instant::now() + deadline)
}

/// The first page target's WebSocket URL, polling `/json/list` on `port`.
///
/// # Errors
/// A response or list of another shape, or no page target by `until`.
pub fn page_socket_until(port: u16, until: Instant) -> Result<String, String> {
    let mut last = String::from("(nothing read)");
    loop {
        if Instant::now() >= until {
            return Err(format!(
                "devtools: no page target in /json/list by the deadline; it read: {last}"
            ));
        }
        if let Some(body) = json_list(port, until)? {
            if let Some(url) = first_page(&body)? {
                return Ok(url);
            }
            last = body;
        }
        thread::sleep(POLL);
    }
}

/// The first page's URL, `None` while there is no page.
fn first_page(body: &str) -> Result<Option<String>, String> {
    let targets: Value = serde_json::from_str(body)
        .map_err(|e| format!("devtools: /json/list is not JSON ({e}): {body}"))?;
    let targets = targets
        .as_array()
        .ok_or_else(|| format!("devtools: /json/list is not a list: {body}"))?;
    let Some(page) = targets.iter().find(|t| t["type"] == "page") else {
        return Ok(None);
    };
    page["webSocketDebuggerUrl"]
        .as_str()
        .map(|url| Some(url.to_owned()))
        .ok_or_else(|| format!("devtools: the page has no webSocketDebuggerUrl (is another client attached?): {page}"))
}

#[cfg(test)]
#[path = "discover_tests.rs"]
mod tests;
