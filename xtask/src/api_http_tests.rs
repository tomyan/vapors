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
