//! The harness server's routes for the session's MCP tools
//! (`api_server.rs`, `api_mcp_inner.rs`): `ask_lead`, the question
//! written for the lead (`api_ask.rs`) and the answer, or why there is
//! none, sent back; and `request_push`, the branch checked, built and
//! pushed (`api_push.rs`) and the PR's URL, or why not, sent back.

use std::os::unix::net::UnixStream;

use crate::api_http::Request;
use crate::api_server::{Reader, Routes, refuse, reply};

/// The path `ask_lead` posts to (`api_mcp_inner.rs`).
pub const ASK: &str = "/api-build/ask";

/// The path `request_push` posts to (`api_mcp_inner.rs`).
pub const PUSH: &str = "/api-build/push";

/// The question whose head is `head`, its body read from `reader`; the
/// answer, waited for up to `api_ask::ANSWER_DEADLINE`.
pub fn asked(stream: &mut UnixStream, (reader, head): (&mut Reader, Request), routes: &Routes) {
    let Some(arguments) = arguments(stream, (reader, head), routes) else {
        return;
    };
    let question = arguments["question"]
        .as_str()
        .map(str::to_owned)
        .filter(|question| !question.trim().is_empty());
    let (Some(question), Some(asks)) = (question, &routes.asks) else {
        return refuse(stream, routes, "an ask_lead call with no question");
    };
    match asks.ask(&question, crate::api_ask::ANSWER_DEADLINE) {
        Ok(answer) => reply(stream, "200 OK", &answer),
        Err(why) => reply(stream, "504 Gateway Timeout", &why),
    }
}

/// The branch whose push is asked for, with the title and body in the
/// request, pushed; the PR's URL, or why not.
pub fn pushed(stream: &mut UnixStream, (reader, head): (&mut Reader, Request), routes: &Routes) {
    let Some(arguments) = arguments(stream, (reader, head), routes) else {
        return;
    };
    let (Some(title), Some(body), Some(push)) = (
        arguments["title"].as_str(),
        arguments["body"].as_str(),
        &routes.push,
    ) else {
        return refuse(
            stream,
            routes,
            "a request_push call without a title and a body",
        );
    };
    match push.push(title, body) {
        Ok(said) => reply(stream, "200 OK", &said),
        Err(why) => reply(stream, "409 Conflict", &why),
    }
}

/// The tool call's arguments, the request's JSON body; `None` once the
/// session has been told why there are none.
fn arguments(
    stream: &mut UnixStream,
    (reader, head): (&mut Reader, Request),
    routes: &Routes,
) -> Option<serde_json::Value> {
    let mut request = head;
    let parsed = crate::api_http::read_body(reader, &mut request).and_then(|()| {
        serde_json::from_slice(&request.body)
            .map_err(|e| format!("a tool call that is not JSON: {e}"))
    });
    parsed.map_err(|why| refuse(stream, routes, &why)).ok()
}

#[cfg(test)]
#[path = "api_server_ask_tests.rs"]
mod tests;
