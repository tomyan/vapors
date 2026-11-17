//! The harness server's `ask_lead` route (`api_server.rs`): the
//! question in the body written for the lead (`api_ask.rs`), and the
//! answer, or why there is none, sent back to the session's MCP server
//! (`api_mcp_inner.rs`).

use std::os::unix::net::UnixStream;

use crate::api_http::Request;
use crate::api_server::{Reader, Routes, refuse, reply};

/// The path `ask_lead` posts to (`api_mcp_inner.rs`).
pub const ASK: &str = "/api-build/ask";

/// The question whose head is `head`, its body read from `reader`; the
/// answer, waited for up to `api_ask::ANSWER_DEADLINE`.
pub fn asked(stream: &mut UnixStream, (reader, head): (&mut Reader, Request), routes: &Routes) {
    let mut request = head;
    if let Err(why) = crate::api_http::read_body(reader, &mut request) {
        return refuse(stream, routes, &why);
    }
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

#[cfg(test)]
#[path = "api_server_ask_tests.rs"]
mod tests;
