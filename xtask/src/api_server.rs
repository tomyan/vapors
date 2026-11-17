//! The harness server (`docs/plans/api-builder.md`): outside the sandbox,
//! on the one socket the session can reach. It reads each request
//! (`api_http.rs`) within a deadline, and answers by its route: the
//! way-in probe; `HEAD /api/hello`, the CLI's connectivity check,
//! answered here with 200, never forwarded and never keyed, and counted
//! (the lead, L13); and, when the run has a key, the API
//! (`api_server_proxy.rs`). A request with no route is refused with 403
//! and logged. Whatever the session sends, the server holds a bounded
//! request, for a bounded time, on a bounded number of connections
//! (review of #467, M3).

use std::io::{BufReader, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::api_http::Request;
use crate::api_server_proxy::Proxy;

/// The path the fences' way-in probe asks for.
pub const PING: &str = "/api-build/ping";

/// How long a request may take to arrive whole: the session's client
/// sends it at once over a local socket (a 32 MB body in well under a
/// second), so 30 s is reached only by a stalled or dripping connection,
/// which is then dropped.
const REQUEST_DEADLINE: Duration = Duration::from_secs(30);

/// The most connections served at once: the CLI holds a few, the MCP
/// server one, the fences one; 64 is room for those and a refusal for a
/// flood, each refused connection logged.
const CONNECTIONS: usize = 64;

/// What the server routes with, and what it saw, for the summary.
#[derive(Default)]
pub struct Routes {
    /// The API route, when the run has a key.
    pub proxy: Option<Proxy>,
    pub refused: Mutex<Vec<String>>,
    /// `HEAD /api/hello` answered here.
    pub hellos: AtomicU64,
}

/// Serves `socket` with `routes` on a thread for as long as the run lasts.
///
/// # Errors
///
/// The socket cannot be bound, naming it.
pub fn serve(socket: &Path, routes: Arc<Routes>) -> Result<(), String> {
    let listener = UnixListener::bind(socket)
        .map_err(|e| format!("binding the harness socket {}: {e}", socket.display()))?;
    thread::spawn(move || accepted(&listener, &routes));
    Ok(())
}

fn accepted(listener: &UnixListener, routes: &Arc<Routes>) {
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
                routes,
                &format!("a connection past the {CONNECTIONS} open"),
            );
            continue;
        }
        let (open, routes) = (Arc::clone(&open), Arc::clone(routes));
        thread::spawn(move || {
            answered(&mut stream, &routes);
            open.fetch_sub(1, Ordering::SeqCst);
        });
    }
}

/// One request read and answered.
pub fn answered(stream: &mut UnixStream, routes: &Routes) {
    let request = match read(stream) {
        Ok(request) => request,
        Err(why) => return refuse(stream, routes, &why),
    };
    match (request.method.as_str(), request.path(), &routes.proxy) {
        ("GET", PING, _) => reply(stream, "200 OK", "pong"),
        ("HEAD", "/api/hello", _) => {
            routes.hellos.fetch_add(1, Ordering::Relaxed);
            reply(stream, "200 OK", "");
        }
        ("POST", "/v1/messages" | "/v1/messages/count_tokens", Some(proxy)) => {
            crate::api_server_proxy::proxied(stream, routes, proxy, &request);
        }
        _ => refuse(
            stream,
            routes,
            &format!("{} {}", request.method, request.target),
        ),
    }
}

/// The request, read whole within [`REQUEST_DEADLINE`].
fn read(stream: &UnixStream) -> Result<Request, String> {
    let reader = Deadlined {
        stream: stream
            .try_clone()
            .map_err(|e| format!("a connection not read: {e}"))?,
        until: Instant::now() + REQUEST_DEADLINE,
    };
    crate::api_http::read(&mut BufReader::new(reader))
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

/// `why` logged and kept for the summary, and the session told, as the
/// API words an error, so the CLI shows it.
pub fn refuse(stream: &mut UnixStream, routes: &Routes, why: &str) {
    eprintln!("api-build: refused: {why}");
    match routes.refused.lock() {
        Ok(mut log) => log.push(why.to_owned()),
        Err(_) => {
            eprintln!("api-build: the refusal log's lock was poisoned; this refusal is only here");
        }
    }
    let body =
        serde_json::json!({"type": "error", "error": {"type": "permission_error", "message": why}});
    reply(stream, "403 Forbidden", &body.to_string());
}

pub fn reply(stream: &mut UnixStream, status: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    if let Err(error) = stream.write_all(response.as_bytes()) {
        eprintln!("api-build: answering `{status}` failed: {error}");
    }
}

#[cfg(test)]
#[path = "api_server_tests.rs"]
mod tests;
