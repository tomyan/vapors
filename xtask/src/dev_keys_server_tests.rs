//! The loopback answer to `/dev/keys.json`, over real HTTP on
//! 127.0.0.1, with temporary key files.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use super::*;
use crate::dev_keys::DevKey;

fn key_file(test: &str, contents: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "xtask-dev-keys-server-{test}-{}",
        std::process::id()
    ));
    std::fs::write(&path, contents).expect("temporary key file");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).expect("its mode");
    path
}

fn openai(file: PathBuf) -> DevKey {
    DevKey {
        provider: "openai".into(),
        file,
    }
}

/// The status line and body of `GET path` on `port`.
fn get(port: u16, path: &str) -> (String, String) {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("the server");
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n"
    )
    .expect("request");
    let mut response = String::new();
    stream.read_to_string(&mut response).expect("response");
    let (head, body) = response.split_once("\r\n\r\n").unwrap_or((&response, ""));
    (
        head.lines().next().unwrap_or_default().to_owned(),
        body.to_owned(),
    )
}

#[test]
fn the_keys_are_answered_as_json_by_provider() {
    // Given
    let file = key_file("json", "sk-test-dev-key-0001\n");
    let port = start(vec![openai(file.clone())]).expect("started");

    // When
    let (status, body) = get(port, KEYS_PATH);

    // Then
    assert_eq!(status, "HTTP/1.1 200 OK");
    assert_eq!(body, r#"{"openai":"sk-test-dev-key-0001"}"#);
    let _ = std::fs::remove_file(file);
}

#[test]
fn each_request_reads_the_file_again() {
    // Given
    let file = key_file("again", "sk-test-dev-key-0001");
    let port = start(vec![openai(file.clone())]).expect("started");
    let _ = get(port, KEYS_PATH);

    // When
    std::fs::write(&file, "sk-test-dev-key-0002").expect("changed");
    let (_, body) = get(port, KEYS_PATH);

    // Then
    assert!(body.contains("sk-test-dev-key-0002"), "{body}");
    let _ = std::fs::remove_file(file);
}

#[test]
fn a_key_file_that_has_gone_is_a_500_naming_the_provider_and_not_the_key() {
    // Given
    let file = key_file("gone", "sk-test-dev-key-0001");
    let port = start(vec![openai(file.clone())]).expect("started");
    let _ = get(port, KEYS_PATH);

    // When
    std::fs::remove_file(&file).expect("removed");
    let (status, body) = get(port, KEYS_PATH);

    // Then
    assert_eq!(status, "HTTP/1.1 500 Internal Server Error");
    // The start-up notice paired the provider with its file.
    assert!(
        body.contains("openai") && body.contains("does not exist"),
        "{body}"
    );
    assert!(!body.contains("sk-test-dev-key"), "{body}");
}

#[test]
fn without_keys_the_answer_is_404() {
    // Given
    let port = start(Vec::new()).expect("started");

    // When
    let (status, _) = get(port, KEYS_PATH);

    // Then
    assert_eq!(status, "HTTP/1.1 404 Not Found");
}

#[test]
fn any_other_path_is_404_and_never_the_keys() {
    // Given
    let file = key_file("other", "sk-test-dev-key-0001");
    let port = start(vec![openai(file.clone())]).expect("started");

    // When
    let (status, body) = get(port, "/dev/keys.json/../index.html");

    // Then
    assert_eq!(status, "HTTP/1.1 404 Not Found");
    assert!(!body.contains("sk-test-dev-key"), "{body}");
    let _ = std::fs::remove_file(file);
}

#[test]
fn an_accept_error_stops_serving_with_a_fail_line_naming_the_fix() {
    // Given
    let connections = vec![Err(std::io::Error::other("too many open files"))].into_iter();

    // When
    let line = serve(connections, &[]);

    // Then
    assert!(line.starts_with("FAIL dev keys:"), "{line}");
    assert!(line.contains("too many open files"), "{line}");
    assert!(line.contains("restart cargo xtask dev"), "{line}");
}
