//! A session with one page: one call at a time, each answered by its id
//! by a deadline, and the events that arrive meanwhile kept in order for
//! [`Session::wait_event`].

use std::collections::VecDeque;
use std::io::ErrorKind;
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use tungstenite::{Message, WebSocket};

/// A WebSocket to a page target.
pub struct Session {
    socket: WebSocket<TcpStream>,
    next: u64,
    events: VecDeque<Value>,
}

impl Session {
    /// Connect to the page of the Chromium using `profile`, by `deadline`
    /// for each of its port, its page target and the connection.
    ///
    /// # Errors
    /// As [`crate::port`], [`crate::page_socket`] and [`Session::connect`].
    pub fn attach(profile: &std::path::Path, deadline: Duration) -> Result<Self, String> {
        let port = crate::port(profile, deadline)?;
        let url = crate::page_socket(port, deadline)?;
        Self::connect(&url, deadline)
    }

    /// Connect to the page target at `url` (`ws://host:port/devtools/page/<id>`).
    ///
    /// # Errors
    /// A URL that is not `ws://`, or no connection and handshake by `deadline`.
    pub fn connect(url: &str, deadline: Duration) -> Result<Self, String> {
        let host = url
            .strip_prefix("ws://")
            .and_then(|rest| rest.split('/').next())
            .ok_or_else(|| format!("devtools: `{url}` is not a ws:// URL"))?;
        let address = host
            .to_socket_addrs()
            .map_err(|e| format!("devtools: {host}: {e}"))?
            .next()
            .ok_or_else(|| format!("devtools: {host} has no address"))?;
        let stream = TcpStream::connect_timeout(&address, deadline)
            .map_err(|e| format!("devtools: connecting to {host}: {e}"))?;
        stream
            .set_read_timeout(Some(deadline))
            .map_err(|e| e.to_string())?;
        let (socket, _) = tungstenite::client(url, stream)
            .map_err(|e| format!("devtools: the WebSocket to {url}: {e}"))?;
        Ok(Self {
            socket,
            next: 0,
            events: VecDeque::new(),
        })
    }

    /// Call `method` and return its result by `deadline`; an error answer
    /// is refused, naming the method.
    ///
    /// # Errors
    /// No answer by `deadline`, an answer that is not JSON, or an error.
    pub fn call(
        &mut self,
        method: &str,
        params: &Value,
        deadline: Duration,
    ) -> Result<Value, String> {
        self.next += 1;
        let id = self.next;
        let request = json!({ "id": id, "method": method, "params": params }).to_string();
        self.socket
            .send(Message::text(request))
            .map_err(|e| format!("devtools: sending {method}: {e}"))?;
        let until = Instant::now() + deadline;
        loop {
            let mut message = self.next_json(
                until,
                &format!("{method} had no answer within {deadline:?}"),
            )?;
            if message["id"].as_u64() == Some(id) {
                if let Some(error) = message.get("error") {
                    return Err(format!("devtools: {method} failed: {error}"));
                }
                return Ok(message["result"].take());
            }
            if message.get("method").is_some() {
                self.events.push_back(message);
            }
        }
    }

    /// The parameters of the next `method` event, kept or arriving, by
    /// `deadline`.
    ///
    /// # Errors
    /// No such event by `deadline`.
    pub fn wait_event(&mut self, method: &str, deadline: Duration) -> Result<Value, String> {
        if let Some(at) = self.events.iter().position(|e| e["method"] == method) {
            return Ok(self
                .events
                .remove(at)
                .map(|mut e| e["params"].take())
                .unwrap_or_default());
        }
        let until = Instant::now() + deadline;
        loop {
            let mut message = self.next_json(
                until,
                &format!("the event {method} did not come within {deadline:?}"),
            )?;
            if message["method"] == method {
                return Ok(message["params"].take());
            }
            if message.get("method").is_some() {
                self.events.push_back(message);
            }
        }
    }

    /// The next text frame as JSON, by `until`; other frames are skipped.
    fn next_json(&mut self, until: Instant, late: &str) -> Result<Value, String> {
        loop {
            let left = until.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Err(format!("devtools: {late}"));
            }
            self.socket
                .get_mut()
                .set_read_timeout(Some(left))
                .map_err(|e| e.to_string())?;
            match self.socket.read() {
                Ok(Message::Text(text)) => {
                    return serde_json::from_str(&text).map_err(|e| {
                        format!("devtools: a message that is not JSON ({e}): {text}")
                    });
                }
                Ok(_) => {}
                Err(tungstenite::Error::Io(e))
                    if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
                Err(e) => return Err(format!("devtools: reading: {e}")),
            }
        }
    }
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
