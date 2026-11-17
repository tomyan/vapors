//! The harness server's API route (`api_server.rs`): `POST /v1/messages`
//! and `/v1/messages/count_tokens` forwarded with the real key
//! (`api_upstream.rs`), each message's usage counted against the budget
//! (`api_budget.rs`), and at most [`IN_FLIGHT`] at once.

use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use crate::api_budget::Budget;
use crate::api_http::Request;
use crate::api_server::{Routes, refuse, reply};
use crate::ux_secret::Secret;

/// The most API requests forwarded at once: the CLI sends one at a time
/// per agent, and a few sub-agents at most; 8 bounds what the session can
/// hold open on the host (each a curl and a staged body of up to 32 MB).
const IN_FLIGHT: usize = 8;

/// The API route's key, budget and staging directory.
pub struct Proxy {
    pub secret: Secret,
    pub budget: Budget,
    /// Private, outside the sandbox: each request's body is staged here.
    pub staging: PathBuf,
    staged: AtomicU64,
    in_flight: AtomicUsize,
}

impl Proxy {
    /// The route with `secret`, `budget`, and bodies staged in `staging`.
    pub fn new(secret: Secret, budget: Budget, staging: PathBuf) -> Self {
        Self {
            secret,
            budget,
            staging,
            staged: AtomicU64::new(0),
            in_flight: AtomicUsize::new(0),
        }
    }
}

/// `request` forwarded, counted when it is a message.
pub fn proxied(stream: &mut UnixStream, routes: &Routes, proxy: &Proxy, request: &Request) {
    if proxy.in_flight.fetch_add(1, Ordering::SeqCst) >= IN_FLIGHT {
        proxy.in_flight.fetch_sub(1, Ordering::SeqCst);
        return refuse(
            stream,
            routes,
            &format!("an API request past the {IN_FLIGHT} in flight"),
        );
    }
    forwarded(stream, routes, proxy, request);
    proxy.in_flight.fetch_sub(1, Ordering::SeqCst);
}

fn forwarded(stream: &mut UnixStream, routes: &Routes, proxy: &Proxy, request: &Request) {
    let model = serde_json::from_slice::<serde_json::Value>(&request.body)
        .ok()
        .and_then(|body| body["model"].as_str().map(str::to_owned))
        .unwrap_or_default();
    let price = match proxy.budget.admit(&model) {
        Ok(price) => price,
        Err(why) => return refuse(stream, routes, &why),
    };
    let number = proxy.staged.fetch_add(1, Ordering::Relaxed);
    let staged = proxy.staging.join(format!("{number}.json"));
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
