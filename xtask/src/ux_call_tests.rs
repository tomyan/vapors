//! The request against a local stand-in: the key arrives as a header and
//! is in no argument; a failed status or connection is refused.

use std::net::TcpListener;
use std::path::PathBuf;

use super::*;
use crate::ux_stub_tests::stand_in;

const KEY: &str = "sk-test-0123456789abcdefghijklmnopqrstuv";

fn files(test: &str) -> (PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("xtask-ux-call-{test}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("made");
    let body = dir.join("body.json");
    std::fs::write(&body, r#"{"model": "m"}"#).expect("written");
    (body, dir.join("response.json"))
}

fn secret() -> Secret {
    Secret::new(KEY.to_owned()).expect("guarded")
}

#[test]
fn the_key_is_sent_as_a_header_and_the_response_is_read() {
    // Given
    let (url, seen) = stand_in("200 OK", r#"{"status": "completed"}"#);
    let (body, response) = files("ok");

    // When
    let answer =
        post(&url, &secret(), &body, &response, Duration::from_secs(10)).expect("answered");

    // Then
    assert_eq!(answer["status"], "completed");
    let (auth, sent) = seen.recv().expect("seen");
    assert_eq!(auth, format!("Authorization: Bearer {KEY}"));
    assert_eq!(sent, r#"{"model": "m"}"#);
}

#[test]
fn the_key_is_in_none_of_curl_s_arguments() {
    // When
    let arguments = arguments(
        "https://api.openai.com/v1/responses",
        Path::new("b.json"),
        Path::new("r.json"),
        DEADLINE,
    );

    // Then
    assert!(
        arguments
            .iter()
            .all(|a| secret().guard("an argument", a).is_ok())
    );
    assert!(arguments.contains(&"@-".to_owned()));
    assert_eq!(
        arguments[0], "-q",
        "curl reads ~/.curlrc unless -q comes first"
    );
}

#[test]
fn a_status_other_than_200_is_refused_with_the_response() {
    // Given
    let (url, _seen) = stand_in(
        "429 Too Many Requests",
        r#"{"error": {"message": "slow down"}}"#,
    );
    let (body, response) = files("429");

    // When
    let refused =
        post(&url, &secret(), &body, &response, Duration::from_secs(10)).expect_err("refused");

    // Then
    assert!(refused.contains("HTTP 429"), "{refused}");
    assert!(!response.exists(), "a failed run's response is kept");
    assert!(refused.contains("slow down"), "{refused}");
}

#[test]
fn a_response_echoing_the_key_is_refused_without_showing_it() {
    // Given
    let (url, _seen) = stand_in(
        "401 Unauthorized",
        r#"{"error": {"message": "Incorrect API key provided: sk-test-0123456789abcdefghijklmnopqrstuv"}}"#,
    );
    let (body, response) = files("echo");

    // When
    let refused =
        post(&url, &secret(), &body, &response, Duration::from_secs(10)).expect_err("refused");

    // Then
    assert!(
        refused.contains("the response would hold part of the API key"),
        "{refused}"
    );
    assert!(!refused.contains(&KEY[KEY.len() - 12..]), "{refused}");
    assert!(!response.exists(), "the response file keeps the key");
    assert!(refused.contains("was deleted"), "{refused}");
}

#[test]
fn a_connection_that_fails_is_refused_with_curl_s_reason() {
    // Given a port nothing listens on
    let port = TcpListener::bind("127.0.0.1:0")
        .expect("bound")
        .local_addr()
        .expect("address")
        .port();
    let (body, response) = files("refused");

    // When
    let refused = post(
        &format!("http://127.0.0.1:{port}/v1/responses"),
        &secret(),
        &body,
        &response,
        Duration::from_secs(10),
    )
    .expect_err("refused");

    // Then
    assert!(refused.starts_with("curl failed"), "{refused}");
}
