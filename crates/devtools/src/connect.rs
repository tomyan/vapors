//! Connecting to a page, and opening a URL in it.

use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::Session;

impl Session {
    /// Open `url` and wait, by `deadline`, for the load of this navigation,
    /// told from any earlier one (the first page's) by its loader id.
    ///
    /// # Errors
    /// A navigation that fails (`errorText`), has no loader, or does not
    /// load by `deadline`.
    pub fn navigate(&mut self, url: &str, deadline: Duration) -> Result<(), String> {
        self.call("Page.enable", &json!({}), deadline)?;
        self.call(
            "Page.setLifecycleEventsEnabled",
            &json!({ "enabled": true }),
            deadline,
        )?;
        let navigated = self.call("Page.navigate", &json!({ "url": url }), deadline)?;
        if let Some(error) = navigated["errorText"].as_str() {
            return Err(format!("devtools: {url} did not load: {error}"));
        }
        let loader = navigated["loaderId"]
            .as_str()
            .ok_or_else(|| format!("devtools: navigating to {url} gave no loaderId"))?
            .to_owned();
        let load = |p: &Value| p["name"] == "load" && p["loaderId"] == loader.as_str();
        self.wait_event_where("Page.lifecycleEvent", &load, deadline)
            .map(|_| ())
    }
}

/// A TCP connection to `url`'s host within `deadline`.
pub(crate) fn tcp(url: &str, deadline: Duration) -> Result<TcpStream, String> {
    let host = url
        .strip_prefix("ws://")
        .and_then(|rest| rest.split('/').next())
        .ok_or_else(|| format!("devtools: `{url}` is not a ws:// URL"))?;
    let address = host
        .to_socket_addrs()
        .map_err(|e| format!("devtools: {host}: {e}"))?
        .next()
        .ok_or_else(|| format!("devtools: {host} has no address"))?;
    TcpStream::connect_timeout(&address, deadline)
        .map_err(|e| format!("devtools: connecting to {host}: {e}"))
}

/// The time left until `until`, refused when none is.
pub(crate) fn left(until: Instant) -> Result<Duration, String> {
    let left = until.saturating_duration_since(Instant::now());
    if left.is_zero() {
        return Err("devtools: the deadline passed".to_owned());
    }
    Ok(left)
}
