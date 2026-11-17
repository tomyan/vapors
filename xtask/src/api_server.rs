//! The harness server (`docs/plans/api-builder.md`): outside the sandbox,
//! on the one socket the session can reach. It reads each request's line
//! and answers by its route; a request it has no route for is refused
//! with 403 and logged. Slice 1 has one route, the probe of the way in;
//! the API, `ask_lead` and `request_push` follow. Whatever the session
//! sends, the server holds a bounded line, for a bounded time, on a
//! bounded number of connections (review of #467, M3).

use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// The path the fences' way-in probe asks for.
pub const PING: &str = "/api-build/ping";

/// The longest request line read: the CLI's are under 100 bytes.
const LINE_LIMIT: u64 = 8 * 1024;

/// How long a request may take to arrive: the session's client sends it
/// at once over a local socket, so 30 s is reached only by a stalled or
/// dripping connection, which is then dropped.
const REQUEST_DEADLINE: Duration = Duration::from_secs(30);

/// The most connections served at once: the CLI holds a few, the MCP
/// server one, the fences one; 64 is room for those and a refusal for a
/// flood, each refused connection logged.
const CONNECTIONS: usize = 64;

/// What the server refused, for the run's summary.
pub type Refused = Arc<Mutex<Vec<String>>>;

/// Serves `socket` on a thread for as long as the run lasts; the
/// requests it refused.
///
/// # Errors
///
/// The socket cannot be bound, naming it.
pub fn serve(socket: &Path) -> Result<Refused, String> {
    let listener = UnixListener::bind(socket)
        .map_err(|e| format!("binding the harness socket {}: {e}", socket.display()))?;
    let refused = Refused::default();
    let log = Arc::clone(&refused);
    thread::spawn(move || accepted(&listener, &log));
    Ok(refused)
}

fn accepted(listener: &UnixListener, refused: &Refused) {
    let open = Arc::new(AtomicUsize::new(0));
    for stream in listener.incoming() {
        let mut stream = match stream {
            Ok(stream) => stream,
            Err(error) => {
                eprintln!(
                    "api-build: a connection to the harness socket was not accepted: {error}"
                );
                thread::sleep(Duration::from_millis(100)); // EMFILE and the like: not a spin
                continue;
            }
        };
        if open.fetch_add(1, Ordering::SeqCst) >= CONNECTIONS {
            open.fetch_sub(1, Ordering::SeqCst);
            refuse(
                &mut stream,
                refused,
                &format!("a connection past the {CONNECTIONS} open"),
            );
            continue;
        }
        let (open, refused) = (Arc::clone(&open), Arc::clone(refused));
        thread::spawn(move || {
            answered(&mut stream, &refused);
            open.fetch_sub(1, Ordering::SeqCst);
        });
    }
}

/// One request read and answered.
pub fn answered(stream: &mut UnixStream, refused: &Refused) {
    let line = match request_line(stream) {
        Ok(line) => line,
        Err(why) => return refuse(stream, refused, &why),
    };
    match line.split(' ').take(2).collect::<Vec<_>>()[..] {
        ["GET", PING] => reply(stream, "200 OK", "pong"),
        _ => refuse(stream, refused, &line),
    }
}

/// The request's first line, read within [`REQUEST_DEADLINE`] and
/// [`LINE_LIMIT`].
fn request_line(stream: &UnixStream) -> Result<String, String> {
    let reader = Deadlined {
        stream: stream
            .try_clone()
            .map_err(|e| format!("a connection not read: {e}"))?,
        until: Instant::now() + REQUEST_DEADLINE,
    };
    let mut line = String::new();
    BufReader::new(reader.take(LINE_LIMIT))
        .read_line(&mut line)
        .map_err(|e| format!("a request line not read within {REQUEST_DEADLINE:?}: {e}"))?;
    if !line.ends_with('\n') {
        return Err(format!(
            "a request line past {LINE_LIMIT} bytes, or cut off"
        ));
    }
    Ok(line.trim_end().to_owned())
}

/// A stream whose reads all end by one instant, however the bytes drip.
struct Deadlined {
    stream: UnixStream,
    until: Instant,
}

impl Read for Deadlined {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let left = self.until.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(std::io::ErrorKind::TimedOut.into());
        }
        self.stream.set_read_timeout(Some(left))?;
        self.stream.read(buffer)
    }
}

fn refuse(stream: &mut UnixStream, refused: &Refused, why: &str) {
    eprintln!("api-build: refused: {why}");
    match refused.lock() {
        Ok(mut log) => log.push(why.to_owned()),
        Err(_) => {
            eprintln!("api-build: the refusal log's lock was poisoned; this refusal is only here");
        }
    }
    reply(
        stream,
        "403 Forbidden",
        "the harness has no route for this request",
    );
}

fn reply(stream: &mut UnixStream, status: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {status}\r\ncontent-type: text/plain\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    if let Err(error) = stream.write_all(response.as_bytes()) {
        eprintln!("api-build: answering `{status}` failed: {error}");
    }
}

#[cfg(test)]
#[path = "api_server_tests.rs"]
mod tests;
