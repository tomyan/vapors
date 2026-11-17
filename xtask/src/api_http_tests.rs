use super::read;

fn parsed(text: &str) -> Result<super::Request, String> {
    read(&mut text.as_bytes())
}

#[test]
fn a_post_with_a_body_is_read_whole() {
    // When
    let request = parsed(
        "POST /v1/messages?beta=true HTTP/1.1\r\nX-Api-Key: dummy\r\ncontent-length: 2\r\n\r\n{}",
    )
    .expect("read");

    // Then
    assert_eq!(
        (
            request.method.as_str(),
            request.path(),
            request.body.as_slice()
        ),
        ("POST", "/v1/messages", b"{}".as_slice())
    );
    assert_eq!(request.header("x-api-key"), Some("dummy"));
}

#[test]
fn a_chunked_body_is_refused() {
    let refused = parsed(
        "POST /v1/messages HTTP/1.1\r\ntransfer-encoding: chunked\r\n\r\n2\r\n{}\r\n0\r\n\r\n",
    );
    assert!(
        refused
            .expect_err("refused")
            .starts_with("a chunked request body")
    );
}

#[test]
fn a_header_line_without_a_colon_is_refused() {
    let refused = parsed("GET / HTTP/1.1\r\nnot a header\r\n\r\n");
    assert!(
        refused
            .expect_err("refused")
            .starts_with("a header line that is not")
    );
}

#[test]
fn a_body_shorter_than_its_length_is_refused() {
    let refused = parsed("POST / HTTP/1.1\r\ncontent-length: 10\r\n\r\n{}");
    assert!(
        refused
            .expect_err("refused")
            .starts_with("reading the 10-byte body")
    );
}

#[test]
fn a_request_with_too_many_headers_is_refused() {
    let headers = "x: y\r\n".repeat(101);
    let refused = parsed(&format!("GET / HTTP/1.1\r\n{headers}\r\n")).expect_err("refused");
    assert_eq!(refused, "a request with more than 100 headers");
}

#[test]
fn a_header_line_past_the_limit_is_refused() {
    let long = "a".repeat(9 * 1024);
    let refused = parsed(&format!("GET / HTTP/1.1\r\nx: {long}\r\n\r\n")).expect_err("refused");
    assert!(
        refused.starts_with("a request head line past 8192 bytes"),
        "{refused}"
    );
}

#[test]
fn a_bare_carriage_return_in_a_header_value_is_refused() {
    let refused =
        parsed("POST / HTTP/1.1\r\nx-a: a\rHost: evil.example\r\n\r\n").expect_err("refused");
    assert!(
        refused.starts_with("a header with a name that is not a token"),
        "{refused}"
    );
}

#[test]
fn a_version_other_than_http_1_1_and_extra_words_are_refused() {
    for line in [
        "POST /v1/messages anything",
        "POST /v1/messages HTTP/1.1 more",
        "POST /v1/messages HTTP/1.0",
    ] {
        let refused = parsed(&format!("{line}\r\n\r\n")).expect_err("refused");
        assert!(
            refused.starts_with("a request line that is not"),
            "{line}: {refused}"
        );
    }
}

#[test]
fn a_header_name_that_is_not_a_token_is_refused() {
    let refused = parsed("GET / HTTP/1.1\r\nx a: b\r\n\r\n").expect_err("refused");
    assert!(
        refused.starts_with("a header with a name that is not a token"),
        "{refused}"
    );
}
