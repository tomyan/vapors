use std::path::Path;

use super::{arguments, passed_head};

#[test]
fn curls_arguments_hold_no_header_and_no_key() {
    // When
    let args = arguments(
        "POST",
        "/v1/messages?beta=true",
        Path::new("/private/1.json"),
    );

    // Then: headers come from stdin (`@-`) only
    let headers: Vec<&String> = args
        .windows(2)
        .filter(|w| w[0] == "--header")
        .map(|w| &w[1])
        .collect();
    assert_eq!(headers, ["@-"]);
    assert!(
        !args.iter().any(|arg| arg.contains("x-api-key")),
        "{args:?}"
    );
    assert_eq!(
        args.last().map(String::as_str),
        Some("https://api.anthropic.com/v1/messages?beta=true")
    );
}

#[test]
fn the_response_head_is_passed_with_its_framing_replaced_by_a_close() {
    // Given
    let response = "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ntransfer-encoding: chunked\r\n\
                    content-length: 5\r\nconnection: keep-alive\r\n\r\nbody";
    let mut reader = response.as_bytes();
    let mut client = Vec::new();

    // When
    let status = passed_head(&mut reader, &mut client).expect("a head");

    // Then
    assert_eq!(status, 200);
    let head = String::from_utf8(client).expect("text");
    assert_eq!(
        head,
        "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\nconnection: close\r\n\r\n"
    );
    assert_eq!(reader, b"body");
}

#[test]
fn a_response_that_is_not_http_is_refused() {
    let refused = passed_head(&mut "curl: (6) no host\n".as_bytes(), &mut Vec::new());
    assert!(
        refused
            .expect_err("refused")
            .starts_with("a response that does not start")
    );
}
