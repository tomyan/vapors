use super::*;

const IMMUTABLE: &str = "public, max-age=31536000, immutable";

#[test]
fn the_status_and_the_two_headers_are_read_whatever_their_case() {
    let headers = "HTTP/2 200 \r\ncontent-type: application/wasm\r\nContent-Encoding: br\r\ncache-control: public, max-age=31536000, immutable\r\n\r\n";
    assert_eq!(
        answer(headers),
        Some(Answer {
            status: 200,
            encoding: Some("br".into()),
            cache: Some(IMMUTABLE.into())
        })
    );
}

#[test]
fn a_file_sent_without_compression_is_refused_naming_it() {
    let plain = Answer {
        status: 200,
        encoding: None,
        cache: Some(IMMUTABLE.into()),
    };
    let refusal = judge("https://h/x_bg.wasm", &plain, IMMUTABLE).expect_err("refused");
    assert!(
        refusal.contains("https://h/x_bg.wasm") && refusal.contains("not br or gzip"),
        "{refusal}"
    );
}

#[test]
fn another_cache_header_or_status_is_refused() {
    let cached = Answer {
        status: 200,
        encoding: Some("gzip".into()),
        cache: Some("max-age=60".into()),
    };
    assert!(
        judge("u", &cached, "no-cache")
            .expect_err("cache")
            .contains("not \"no-cache\"")
    );
    let missing = Answer {
        status: 404,
        encoding: None,
        cache: None,
    };
    assert!(
        judge("u", &missing, "no-cache")
            .expect_err("status")
            .contains("answered 404")
    );
}

#[test]
fn a_compressed_answer_with_its_cache_header_passes() {
    let good = Answer {
        status: 200,
        encoding: Some("br".into()),
        cache: Some("no-cache".into()),
    };
    assert_eq!(judge("u", &good, "no-cache"), Ok(()));
}
