//! A session against a fake `DevTools` server that plays a script.

use std::net::TcpListener;
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use tungstenite::Message;

use super::*;

const SHORT: Duration = Duration::from_millis(400);

/// A server that answers its first message with `script(id)`'s frames,
/// `gap` apart.
fn server(script: fn(u64) -> Vec<Message>) -> String {
    server_spaced(script, Duration::ZERO)
}

fn server_spaced(script: fn(u64) -> Vec<Message>, gap: Duration) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a port");
    let port = listener.local_addr().expect("address").port();
    thread::spawn(move || {
        let (stream, _) = listener.accept().expect("a client");
        let mut socket = tungstenite::accept(stream).expect("a handshake");
        let Ok(request) = socket.read() else { return };
        let request: Value =
            serde_json::from_str(request.to_text().unwrap_or("{}")).unwrap_or_default();
        for frame in script(request["id"].as_u64().unwrap_or(0)) {
            if socket.send(frame).is_err() {
                return;
            }
            thread::sleep(gap);
        }
        thread::sleep(Duration::from_secs(2));
    });
    format!("ws://127.0.0.1:{port}/devtools/page/1")
}

fn text(value: &Value) -> Message {
    Message::text(value.to_string())
}

#[test]
fn a_call_returns_its_own_answer_and_keeps_the_events_before_it() {
    // Given
    let url = server(|id| {
        vec![
            text(&json!({ "method": "Page.loadEventFired", "params": { "timestamp": 1 } })),
            text(&json!({ "id": id + 100, "result": {} })),
            text(&json!({ "id": id, "result": { "data": "frame" } })),
        ]
    });
    let mut session = Session::connect(&url, SHORT).expect("connects");

    // When
    let result = session
        .call("Page.captureScreenshot", &json!({}), SHORT)
        .expect("answers");

    // Then
    assert_eq!(result["data"], "frame");
    let event = session
        .wait_event("Page.loadEventFired", SHORT)
        .expect("kept");
    assert_eq!(event["timestamp"], 1);
}

#[test]
fn a_protocol_error_is_refused_naming_the_method() {
    // Given
    let url = server(|id| {
        vec![text(
            &json!({ "id": id, "error": { "code": -32000, "message": "No node" } }),
        )]
    });
    let mut session = Session::connect(&url, SHORT).expect("connects");

    // When
    let refusal = session
        .call("DOM.focus", &json!({}), SHORT)
        .expect_err("refused");

    // Then
    assert!(refusal.contains("DOM.focus failed"), "{refusal}");
    assert!(refusal.contains("No node"), "{refusal}");
}

#[test]
fn a_call_with_no_answer_fails_at_its_deadline_even_with_pings_arriving() {
    // Given: a ping every 50 ms for 2 s, never the answer
    let pings = |_| (0..40).map(|_| Message::Ping(Vec::new().into())).collect();
    let url = server_spaced(pings, Duration::from_millis(50));
    let mut session = Session::connect(&url, SHORT).expect("connects");

    // When
    let started = Instant::now();
    let refusal = session
        .call("Page.navigate", &json!({}), SHORT)
        .expect_err("no answer");

    // Then
    assert!(started.elapsed() < SHORT * 4, "{:?}", started.elapsed());
    assert!(
        refusal.contains("Page.navigate had no answer within"),
        "{refusal}"
    );
}

#[test]
fn an_event_that_never_comes_fails_at_its_deadline_naming_it() {
    // Given
    let url = server(|id| vec![text(&json!({ "id": id, "result": {} }))]);
    let mut session = Session::connect(&url, SHORT).expect("connects");
    session
        .call("Page.enable", &json!({}), SHORT)
        .expect("answers");

    // When
    let refusal = session
        .wait_event("Page.loadEventFired", SHORT)
        .expect_err("never");

    // Then
    assert!(refusal.contains("Page.loadEventFired"), "{refusal}");
}

#[test]
fn an_answer_that_is_not_json_is_refused_quoting_it() {
    let url = server(|_| vec![Message::text("not json")]);
    let mut session = Session::connect(&url, SHORT).expect("connects");
    let refusal = session
        .call("Page.enable", &json!({}), SHORT)
        .expect_err("refused");
    assert!(refusal.contains("not json"), "{refusal}");
}

#[test]
fn messages_of_a_shape_not_recognised_are_refused_quoting_them() {
    type Script = fn(u64) -> Vec<Message>;
    let cases: [(Script, &str); 3] = [
        (
            |id| vec![text(&json!({ "id": id }))],
            "neither result nor error",
        ),
        (
            |_| vec![text(&json!({ "params": {} }))],
            "neither id nor method",
        ),
        (|_| vec![Message::Binary(vec![1, 2].into())], "binary"),
    ];
    for (script, needle) in cases {
        let url = server(script);
        let mut session = Session::connect(&url, SHORT).expect("connects");
        let refusal = session
            .call("Page.enable", &json!({}), SHORT)
            .expect_err(needle);
        assert!(refusal.contains(needle), "wanted {needle} in: {refusal}");
    }
}
