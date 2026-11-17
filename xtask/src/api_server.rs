//! The harness server (`docs/plans/api-builder.md`): outside the sandbox,
//! on the one socket the session can reach. It reads each request
//! (`api_http.rs`) and answers by its route: the way-in probe, and, when
//! the run has a key, `POST /v1/messages` and `/v1/messages/count_tokens`
//! forwarded to the API with it (`api_upstream.rs`), counted against the
//! budget (`api_budget.rs`); and `ask_lead`'s questions (`api_ask.rs`).
//! A request with no route is refused with 403 and logged.

use std::io::{BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::api_budget::Budget;
use crate::api_http::Request;
use crate::ux_secret::Secret;

/// The path the fences' way-in probe asks for.
pub const PING: &str = "/api-build/ping";

/// The path `ask_lead` posts to (`api_mcp_inner.rs`).
const ASK: &str = "/api-build/ask";

/// How long a request may take to arrive whole: the session's client
/// sends it at once over a local socket, so 30 s is reached only by a
/// stalled connection.
const REQUEST_DEADLINE: Duration = Duration::from_secs(30);

/// The API route's key, budget and staging directory.
pub struct Proxy {
    pub secret: Secret,
    pub budget: Budget,
    /// Private, outside the sandbox: each request's body is staged here.
    pub staging: PathBuf,
}

/// What the server routes with, and what it refused, for the summary.
#[derive(Default)]
pub struct Routes {
    pub proxy: Option<Proxy>,
    /// Where `ask_lead`'s questions go (`api_ask.rs`).
    pub asks: Option<crate::api_ask::Asks>,
    pub refused: Mutex<Vec<String>>,
    staged: AtomicU64,
}

impl Routes {
    /// Routes with the API forwarded through `proxy` and questions
    /// written for the lead in `asks`.
    pub fn with(proxy: Proxy, asks: crate::api_ask::Asks) -> Self {
        Self {
            proxy: Some(proxy),
            asks: Some(asks),
            ..Self::default()
        }
    }
}

/// Serves `socket` with `routes` on a thread for as long as the run lasts.
///
/// # Errors
///
/// The socket cannot be bound, naming it.
pub fn serve(socket: &Path, routes: Arc<Routes>) -> Result<(), String> {
    let listener = UnixListener::bind(socket)
        .map_err(|e| format!("binding the harness socket {}: {e}", socket.display()))?;
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let routes = Arc::clone(&routes);
            thread::spawn(move || answered(stream, &routes));
        }
    });
    Ok(())
}

fn answered(mut stream: UnixStream, routes: &Routes) {
    let _ = stream.set_read_timeout(Some(REQUEST_DEADLINE));
    let request = stream
        .try_clone()
        .map_err(|e| e.to_string())
        .and_then(|reader| crate::api_http::read(&mut BufReader::new(reader)));
    let request = match request {
        Ok(request) => request,
        Err(why) => return refuse(&mut stream, routes, &format!("a request not read: {why}")),
    };
    let line = format!("{} {}", request.method, request.target);
    match (request.method.as_str(), request.path(), &routes.proxy) {
        ("GET", PING, _) => reply(&mut stream, "200 OK", "pong"),
        ("POST", "/v1/messages" | "/v1/messages/count_tokens", Some(proxy)) => {
            proxied(&mut stream, routes, proxy, &request);
        }
        ("POST", ASK, _) if routes.asks.is_some() => asked(&mut stream, routes, &request),
        _ => refuse(&mut stream, routes, &line),
    }
}

/// `ask_lead`: the question in the body, the answer or why there is none.
fn asked(stream: &mut UnixStream, routes: &Routes, request: &Request) {
    let question = serde_json::from_slice::<serde_json::Value>(&request.body)
        .ok()
        .and_then(|body| body["question"].as_str().map(str::to_owned))
        .filter(|question| !question.trim().is_empty());
    let (Some(question), Some(asks)) = (question, &routes.asks) else {
        return refuse(stream, routes, "an ask_lead call with no question");
    };
    match asks.ask(&question, crate::api_ask::ANSWER_DEADLINE) {
        Ok(answer) => reply(stream, "200 OK", &answer),
        Err(why) => reply(stream, "504 Gateway Timeout", &why),
    }
}

/// `request` forwarded, counted when it is a message.
fn proxied(stream: &mut UnixStream, routes: &Routes, proxy: &Proxy, request: &Request) {
    let model = serde_json::from_slice::<serde_json::Value>(&request.body)
        .ok()
        .and_then(|body| body["model"].as_str().map(str::to_owned))
        .unwrap_or_default();
    let price = match proxy.budget.admit(&model) {
        Ok(price) => price,
        Err(why) => return refuse(stream, routes, &why),
    };
    let staged = proxy.staging.join(format!(
        "{}.json",
        routes.staged.fetch_add(1, Ordering::Relaxed)
    ));
    match crate::api_upstream::forward(request, &proxy.secret, &staged, stream) {
        Ok(forwarded)
            if request.path() == "/v1/messages" && (200..300).contains(&forwarded.status) =>
        {
            let why = format!("a {} from /v1/messages with no usage", forwarded.status);
            proxy.budget.charge(price, forwarded.usage, &why);
        }
        Ok(_) => {}
        Err(why) => {
            eprintln!(
                "api-build: forwarding `{} {}` failed: {why}",
                request.method, request.target
            );
            reply(stream, "502 Bad Gateway", &why);
        }
    }
}

fn refuse(stream: &mut UnixStream, routes: &Routes, why: &str) {
    eprintln!("api-build: refused: {why}");
    if let Ok(mut log) = routes.refused.lock() {
        log.push(why.to_owned());
    }
    let body =
        serde_json::json!({"type": "error", "error": {"type": "permission_error", "message": why}});
    reply(stream, "403 Forbidden", &body.to_string());
}

fn reply(stream: &mut UnixStream, status: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    if let Err(error) = stream.write_all(response.as_bytes()) {
        eprintln!("api-build: answering `{status}` failed: {error}");
    }
}
