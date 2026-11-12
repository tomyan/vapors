use super::*;

fn good(content_type: &str, cache: &str) -> Answer {
    Answer {
        status: 200,
        encoding: Some("br".into()),
        cache: Some(cache.into()),
        content_type: Some(content_type.into()),
        vary: Some("accept-encoding".into()),
    }
}

const WASM: &str = "touche-web-0123456789abcdef_bg.wasm";

#[test]
fn the_status_and_headers_are_read_whatever_their_case() {
    let headers = "HTTP/2 200 \r\nContent-Type: application/wasm\r\nContent-Encoding: br\r\nVary: Accept-Encoding\r\ncache-control: no-cache\r\n\r\n";
    assert_eq!(
        answer(headers),
        Some(Answer {
            status: 200,
            encoding: Some("br".into()),
            cache: Some("no-cache".into()),
            content_type: Some("application/wasm".into()),
            vary: Some("Accept-Encoding".into()),
        })
    );
}

#[test]
fn hashed_names_are_immutable_and_the_rest_no_cache_each_with_its_type() {
    let cases = [
        (WASM, "application/wasm", IMMUTABLE),
        (
            "touche-web-0123456789abcdef.js",
            "text/javascript",
            IMMUTABLE,
        ),
        ("app-0123456789abcdef.css", "text/css", IMMUTABLE),
        ("encode_worker_bg.wasm", "application/wasm", NO_CACHE),
        ("shell.css", "text/css", NO_CACHE),
        (
            "touche-web-0123456789abcdeX.js",
            "text/javascript",
            NO_CACHE,
        ),
        ("index.html", "text/html", NO_CACHE),
    ];
    for (path, content_type, cache) in cases {
        assert_eq!(
            expect_for(path),
            Expect {
                content_type,
                cache
            },
            "{path}"
        );
    }
}

#[test]
fn a_good_answer_passes() {
    let expect = expect_for(WASM);
    assert_eq!(
        judge("u", &good("application/wasm", IMMUTABLE), &expect),
        Ok(())
    );
}

#[test]
fn each_wrong_header_is_refused_naming_it() {
    let expect = expect_for(WASM);
    let mut cases: Vec<(Answer, &str)> = Vec::new();
    cases.push((
        Answer {
            status: 404,
            ..good("application/wasm", IMMUTABLE)
        },
        "answered 404",
    ));
    cases.push((
        Answer {
            encoding: None,
            ..good("application/wasm", IMMUTABLE)
        },
        "not br or gzip",
    ));
    cases.push((
        good("application/octet-stream", IMMUTABLE),
        "not application/wasm",
    ));
    cases.push((
        Answer {
            vary: None,
            ..good("application/wasm", IMMUTABLE)
        },
        "without Accept-Encoding",
    ));
    cases.push((good("application/wasm", NO_CACHE), "Cache-Control"));
    for (got, said) in cases {
        let refusal = judge("https://h/x_bg.wasm", &got, &expect).expect_err(said);
        assert!(
            refusal.starts_with("https://h/x_bg.wasm") && refusal.contains(said),
            "{said}: {refusal}"
        );
    }
}

#[test]
fn a_content_type_with_parameters_is_read_by_its_type() {
    let expect = expect_for("index.html");
    assert_eq!(
        judge("u", &good("text/html; charset=utf-8", NO_CACHE), &expect),
        Ok(())
    );
}

#[test]
fn the_first_relative_script_is_found() {
    let index = r#"<script src="https://x/y.js"></script><script defer src="./browser.js?v=1"></script><script src="b.js"></script>"#;
    assert_eq!(first_script(index), Some("browser.js?v=1".to_owned()));
    assert_eq!(first_script("<script>inline()</script>"), None);
}
