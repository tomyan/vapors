//! A session with one page: one call at a time, each answered by its id
//! by a deadline, and the events that arrive meanwhile kept in order for
//! [`Session::wait_event`]. A message of a shape the protocol does not
//! have is refused, quoting it.
//!
//! Declared limit: events nobody waits for are kept for the session's
//! life. Sessions here last one check or one oracle run.

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

/// A message from the page: the answer to a call, or an event.
enum Incoming {
    Answer(u64, Value),
    Event(Value),
}

impl Session {
    /// Connect to the page of the Chromium using `profile`, all within
    /// `deadline`; `alive` is asked on each wait and stops it when
    /// Chromium has exited.
    ///
    /// # Errors
    /// As [`crate::port`], [`crate::page_socket`] and [`Session::connect`].
    pub fn attach(
        profile: &std::path::Path,
        deadline: Duration,
        alive: &mut dyn FnMut() -> Result<(), String>,
    ) -> Result<Self, String> {
        let until = Instant::now() + deadline;
        let port = crate::discover::port_until(profile, until, alive)?;
        let url = crate::discover::page_socket_until(port, until)?;
        Self::connect(&url, until.saturating_duration_since(Instant::now()))
    }

    /// Connect to the page target at `url` (`ws://host:port/devtools/page/<id>`)
    /// within `deadline`, the connection and the handshake together.
    ///
    /// # Errors
    /// A URL that is not `ws://`, or no connection and handshake by `deadline`.
    pub fn connect(url: &str, deadline: Duration) -> Result<Self, String> {
        let until = Instant::now() + deadline;
        let stream = tcp(url, deadline)?;
        stream
            .set_read_timeout(Some(left(until)?))
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
    /// No answer by `deadline`, a message of another shape, or an error.
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
            match self.next_message(
                until,
                &format!("{method} had no answer within {deadline:?}"),
            )? {
                Incoming::Answer(answered, message) if answered == id => {
                    return answer(method, message);
                }
                Incoming::Answer(..) => {}
                Incoming::Event(event) => self.events.push_back(event),
            }
        }
    }

    /// The parameters of the next `method` event, kept or arriving, by
    /// `deadline`.
    ///
    /// # Errors
    /// No such event by `deadline`, or a message of another shape.
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
            match self.next_message(
                until,
                &format!("the event {method} did not come within {deadline:?}"),
            )? {
                Incoming::Event(mut event) if event["method"] == method => {
                    return Ok(event["params"].take());
                }
                Incoming::Event(event) => self.events.push_back(event),
                Incoming::Answer(..) => {}
            }
        }
    }

    /// The next message, by `until`. Pings and pongs are skipped; a binary
    /// frame, or text that is neither an answer nor an event, is refused.
    fn next_message(&mut self, until: Instant, late: &str) -> Result<Incoming, String> {
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
                Ok(Message::Text(text)) => return incoming(&text),
                Ok(Message::Binary(bytes)) => {
                    return Err(format!(
                        "devtools: a binary frame of {} bytes, which the protocol does not send",
                        bytes.len()
                    ));
                }
                Ok(_) => {}
                Err(tungstenite::Error::Io(e))
                    if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
                Err(e) => return Err(format!("devtools: reading: {e}")),
            }
        }
    }
}

/// A text frame as an answer (an `id`) or an event (a `method`).
fn incoming(text: &str) -> Result<Incoming, String> {
    let message: Value = serde_json::from_str(text)
        .map_err(|e| format!("devtools: a message that is not JSON ({e}): {text}"))?;
    if let Some(id) = message["id"].as_u64() {
        return Ok(Incoming::Answer(id, message));
    }
    if message["method"].is_string() {
        return Ok(Incoming::Event(message));
    }
    Err(format!(
        "devtools: a message with neither id nor method: {text}"
    ))
}

/// An answer's result, or its error refused.
fn answer(method: &str, mut message: Value) -> Result<Value, String> {
    if let Some(error) = message.get("error") {
        return Err(format!("devtools: {method} failed: {error}"));
    }
    match message.get_mut("result") {
        Some(result) => Ok(result.take()),
        None => Err(format!(
            "devtools: {method}'s answer has neither result nor error: {message}"
        )),
    }
}

/// A TCP connection to `url`'s host within `deadline`.
fn tcp(url: &str, deadline: Duration) -> Result<TcpStream, String> {
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

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
