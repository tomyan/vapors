//! The harness server's API route (`api_server.rs`): `POST /v1/messages`
//! and `/v1/messages/count_tokens` forwarded with the real key
//! (`api_upstream.rs`), each message's usage counted against the budget
//! (`api_budget.rs`), and at most [`IN_FLIGHT`] at once.

use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use crate::api_budget::Budget;
use crate::api_http::Request;
use crate::api_server::{Reader, Routes, refuse, reply};
use crate::api_upstream::Staged;
use crate::api_usage::Counted;
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
    /// How many API requests are in flight now.
    pub fn in_flight(&self) -> usize {
        self.in_flight.load(Ordering::SeqCst)
    }

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

/// The request whose head is `head`, its body read from `reader` once
/// there is room for it, judged (`api_request_policy.rs`), admitted
/// against the budget at its worst case, forwarded, and settled.
pub fn proxied(
    stream: &mut UnixStream,
    (reader, head): (&mut Reader, Request),
    routes: &Routes,
    proxy: &Proxy,
) {
    if proxy.in_flight.fetch_add(1, Ordering::SeqCst) >= IN_FLIGHT {
        proxy.in_flight.fetch_sub(1, Ordering::SeqCst);
        return refuse(
            stream,
            routes,
            &format!("an API request past the {IN_FLIGHT} in flight"),
        );
    }
    let mut request = head;
    match crate::api_http::read_body(reader, &mut request) {
        Ok(()) => judged(stream, routes, proxy, &request),
        Err(why) => refuse(stream, routes, &why),
    }
    proxy.in_flight.fetch_sub(1, Ordering::SeqCst);
}

fn judged(stream: &mut UnixStream, routes: &Routes, proxy: &Proxy, request: &Request) {
    let demand = match crate::api_request_policy::judged(&request.target, &request.body) {
        Ok(demand) => demand,
        Err(why) => return refuse(stream, routes, &why),
    };
    let ticket = match demand
        .as_ref()
        .map(|demand| proxy.budget.admit(demand))
        .transpose()
    {
        Ok(ticket) => ticket,
        Err(why) => return refuse(stream, routes, &why),
    };
    let counted = forwarded(stream, proxy, request);
    if let (Some(ticket), Some(demand)) = (ticket, demand) {
        let counted = counted
            .unwrap_or_else(|| Counted::Whole(crate::api_prices::Usage::default(), demand.model));
        proxy.budget.settle(ticket, &counted);
    }
}

/// `request` forwarded; what it counted, or `None` for an error the API
/// answered whole, which it bills nothing for. One that never got a head
/// counts as cut, charged at its worst case.
fn forwarded(stream: &mut UnixStream, proxy: &Proxy, request: &Request) -> Option<Counted> {
    let number = proxy.staged.fetch_add(1, Ordering::Relaxed);
    let sent = Staged::new(proxy.staging.join(format!("{number}.json")), &request.body)
        .and_then(|staged| crate::api_upstream::forward(request, &proxy.secret, &staged, stream));
    match sent {
        Ok(forwarded)
            if !(200..300).contains(&forwarded.status)
                && !matches!(forwarded.counted, Counted::Cut(_)) =>
        {
            None
        }
        Ok(forwarded) => Some(forwarded.counted),
        Err(why) => {
            let target = crate::api_refusals::escaped(&request.target);
            eprintln!(
                "api-build: forwarding `{} {target}` failed: {why}",
                request.method
            );
            reply(stream, "502 Bad Gateway", &why);
            Some(Counted::Cut(why))
        }
    }
}
