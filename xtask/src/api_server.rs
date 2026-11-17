//! The harness server (`docs/plans/api-builder.md`): outside the sandbox,
//! on the one socket the session can reach. It reads each request's head
//! and answers by its route; a request it has no route for is refused
//! with 403 and logged. Slice 1 has one route, the probe of the way in;
//! the API, `ask_lead` and `request_push` follow.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// The path the fences' way-in probe asks for.
pub const PING: &str = "/api-build/ping";

/// How long a request's head may take to arrive: the session's client
/// sends it at once, so 30 s is reached only by a stalled connection.
const HEAD_DEADLINE: Duration = Duration::from_secs(30);

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
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let log = Arc::clone(&log);
            thread::spawn(move || answered(stream, &log));
        }
    });
    Ok(refused)
}

fn answered(stream: UnixStream, refused: &Refused) {
    let _ = stream.set_read_timeout(Some(HEAD_DEADLINE));
    let Ok(reader) = stream.try_clone() else {
        return;
    };
    let mut line = String::new();
    if BufReader::new(reader).read_line(&mut line).is_err() {
        refuse(stream, refused, "a request whose head did not arrive");
        return;
    }
    let request = line.trim().to_owned();
    match request.split(' ').take(2).collect::<Vec<_>>()[..] {
        ["GET", PING] => reply(stream, "200 OK", "pong"),
        _ => refuse(stream, refused, &request),
    }
}

fn refuse(stream: UnixStream, refused: &Refused, request: &str) {
    eprintln!("api-build: refused `{request}`: the harness has no route for it");
    if let Ok(mut log) = refused.lock() {
        log.push(request.to_owned());
    }
    reply(
        stream,
        "403 Forbidden",
        "the harness has no route for this request",
    );
}

fn reply(mut stream: UnixStream, status: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {status}\r\ncontent-type: text/plain\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    if let Err(error) = stream.write_all(response.as_bytes()) {
        eprintln!("api-build: answering `{status}` failed: {error}");
    }
}
